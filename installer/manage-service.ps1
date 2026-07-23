[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet("Install", "Status", "Start", "Stop", "Disable", "Enable", "Uninstall")]
    [string]$Action,

    [string]$ExecutablePath = (
        Join-Path $PSScriptRoot "..\target\x86_64-pc-windows-msvc\release\cuadra-pos-agent.exe"
    ),

    [switch]$RemoveData
)

$ErrorActionPreference = "Stop"
$serviceName = "CuadraPosAgent"
$displayName = "Cuadra POS Agent"
$installDirectory = Join-Path $env:ProgramFiles "Cuadra POS Agent"
$installedExecutable = Join-Path $installDirectory "cuadra-pos-agent.exe"
$dataDirectory = Join-Path $env:ProgramData "Cuadra ERP\Cuadra POS Agent"
$testerShortcut = Join-Path $env:ProgramData "Microsoft\Windows\Start Menu\Programs\Cuadra POS Agent.url"
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function Test-IsAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Assert-Administrator {
    if (-not (Test-IsAdministrator)) {
        throw "Esta acción requiere PowerShell como administrador."
    }
}

function Get-AgentService {
    return Get-Service -Name $serviceName -ErrorAction SilentlyContinue
}

function Wait-ServiceState([string]$State, [int]$Seconds = 20) {
    $service = Get-AgentService
    if ($service) {
        $service.WaitForStatus($State, [TimeSpan]::FromSeconds($Seconds))
    }
}

function Get-AgentEndpoint {
    $configuration = Join-Path $dataDirectory "config.json"
    if (Test-Path $configuration) {
        $settings = Get-Content -LiteralPath $configuration -Raw | ConvertFrom-Json
        if ($settings.server.tlsEnabled) {
            return "https://localhost:$($settings.server.port)"
        }
        return "http://localhost:$($settings.server.port)"
    }
    return "http://localhost:17442"
}

function Set-AgentDataPermissions {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    $logsDirectory = Join-Path $dataDirectory "logs"
    $grants = @(
        '*S-1-5-18:(OI)(CI)F'
        '*S-1-5-32-544:(OI)(CI)F'
        "${currentUser}:(OI)(CI)M"
    )
    & icacls.exe $dataDirectory /inheritance:e /grant:r $grants | Out-Null
    $dataExitCode = $LASTEXITCODE
    & icacls.exe $logsDirectory /inheritance:e /grant:r $grants /T /C | Out-Null
    $logsExitCode = $LASTEXITCODE
    if ($dataExitCode -ne 0 -or $logsExitCode -ne 0) {
        Write-Warning "No se pudieron actualizar todos los permisos de $dataDirectory."
    }
}

function Show-AgentStartupDiagnostics {
    Write-Warning "Diagnóstico de inicio de Cuadra POS Agent:"
    & sc.exe queryex $serviceName

    $configuration = Join-Path $dataDirectory "config.json"
    if (Test-Path -LiteralPath $configuration) {
        Write-Warning "Configuración: $configuration"
        $settings = Get-Content -LiteralPath $configuration -Raw | ConvertFrom-Json
        $ports = @($settings.server.port)
        if ($settings.server.tlsEnabled -and $null -ne $settings.server.httpPort) {
            $ports += $settings.server.httpPort
        }
        foreach ($port in $ports | Select-Object -Unique) {
            $listeners = Get-NetTCPConnection -State Listen -LocalPort $port -ErrorAction SilentlyContinue
            foreach ($listener in $listeners) {
                $process = Get-Process -Id $listener.OwningProcess -ErrorAction SilentlyContinue
                Write-Warning "Puerto $port ocupado por PID $($listener.OwningProcess) ($($process.ProcessName))."
            }
        }
    }

    $latestLog = Get-ChildItem -LiteralPath (Join-Path $dataDirectory "logs") `
        -Filter "agent.log*" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($latestLog) {
        Write-Warning "Últimas líneas de $($latestLog.FullName):"
        Get-Content -LiteralPath $latestLog.FullName -Tail 30
    }
    else {
        $startupLog = Join-Path $dataDirectory "startup-error.log"
        if (Test-Path -LiteralPath $startupLog) {
            Write-Warning "Errores de inicio en ${startupLog}:"
            Get-Content -LiteralPath $startupLog -Tail 30
        }
        else {
            Write-Warning "No se encontró agent.log. Pruebe el ejecutable manualmente:"
            Write-Warning ('& "{0}" --no-browser' -f $installedExecutable)
        }
    }
}

switch ($Action) {
    "Install" {
        Assert-Administrator
        $source = (Resolve-Path $ExecutablePath).Path
        New-Item -ItemType Directory -Path $installDirectory -Force | Out-Null
        New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $dataDirectory "logs") -Force | Out-Null
        Set-AgentDataPermissions

        $existing = Get-AgentService
        if ($existing) {
            if ($existing.Status -ne "Stopped") {
                Stop-Service -Name $serviceName -Force
                Wait-ServiceState "Stopped"
            }
        }

        Copy-Item -LiteralPath $source -Destination $installedExecutable -Force
        $configuration = Join-Path $dataDirectory "config.json"
        if (-not (Test-Path $configuration)) {
            Copy-Item -LiteralPath (Join-Path $projectRoot "config\config.example.json") `
                -Destination $configuration
        }

        $settings = Get-Content -LiteralPath $configuration -Raw | ConvertFrom-Json
        if ($settings.server.tlsEnabled) {
            & $installedExecutable --install-ca
            if ($LASTEXITCODE -ne 0) {
                Write-Warning "No se pudo preparar el certificado local. Se continuará únicamente por HTTP."
                $settings.server.tlsEnabled = $false
                if ($null -ne $settings.server.httpPort) {
                    $settings.server.port = $settings.server.httpPort
                }
                $json = $settings | ConvertTo-Json -Depth 10
                [IO.File]::WriteAllText($configuration, $json, [Text.UTF8Encoding]::new($false))
            }
        }

        $agentEndpoint = Get-AgentEndpoint

        if (-not $existing) {
            $binaryPath = '"{0}" --service' -f $installedExecutable
            New-Service -Name $serviceName -BinaryPathName $binaryPath `
                -DisplayName $displayName -Description "Agente local para hardware de Cuadra POS." `
                -StartupType Automatic | Out-Null
        }
        else {
            Set-Service -Name $serviceName -StartupType Automatic
        }

        & sc.exe config $serviceName binPath= ('"{0}" --service' -f $installedExecutable) start= auto | Out-Null

        & sc.exe failure $serviceName reset= 86400 actions= restart/5000/restart/15000/restart/30000 | Out-Null
        & sc.exe failureflag $serviceName 1 | Out-Null
        & sc.exe sidtype $serviceName unrestricted | Out-Null
        @(
            "[InternetShortcut]"
            "URL=$agentEndpoint/tester"
            "IconFile=$installedExecutable"
            "IconIndex=0"
        ) | Set-Content -LiteralPath $testerShortcut -Encoding ascii
        try {
            Start-Service -Name $serviceName
            Wait-ServiceState "Running"
        }
        catch {
            Show-AgentStartupDiagnostics
            throw
        }
        Write-Host "Cuadra POS Agent instalado y ejecutándose en segundo plano."
        Write-Host "Interfaz: $agentEndpoint/tester"
    }

    "Status" {
        $service = Get-AgentService
        if (-not $service) {
            Write-Host "Cuadra POS Agent no está instalado."
            exit 1
        }
        $service | Select-Object Name, DisplayName, Status, StartType
        try {
            $agentEndpoint = Get-AgentEndpoint
            $health = Invoke-RestMethod "$agentEndpoint/health"
            $health | Format-List
        }
        catch {
            Write-Warning "El servicio existe pero el endpoint no respondió: $($_.Exception.Message)"
        }
    }

    "Start" {
        Assert-Administrator
        try {
            Start-Service -Name $serviceName
            Wait-ServiceState "Running"
        }
        catch {
            Show-AgentStartupDiagnostics
            throw
        }
        Write-Host "Servicio iniciado."
    }

    "Stop" {
        Assert-Administrator
        Stop-Service -Name $serviceName -Force
        Wait-ServiceState "Stopped"
        Write-Host "Servicio detenido hasta el próximo inicio manual o reinicio de Windows."
    }

    "Disable" {
        Assert-Administrator
        $service = Get-AgentService
        if ($service -and $service.Status -ne "Stopped") {
            Stop-Service -Name $serviceName -Force
            Wait-ServiceState "Stopped"
        }
        Set-Service -Name $serviceName -StartupType Disabled
        Write-Host "Servicio detenido y deshabilitado permanentemente."
    }

    "Enable" {
        Assert-Administrator
        Set-Service -Name $serviceName -StartupType Automatic
        Start-Service -Name $serviceName
        Wait-ServiceState "Running"
        Write-Host "Servicio habilitado y ejecutándose."
    }

    "Uninstall" {
        Assert-Administrator
        $service = Get-AgentService
        if ($service) {
            if ($service.Status -ne "Stopped") {
                Stop-Service -Name $serviceName -Force
                Wait-ServiceState "Stopped"
            }
            & sc.exe delete $serviceName | Out-Null
            Start-Sleep -Seconds 1
        }
        if (Test-Path $installDirectory) {
            Remove-Item -LiteralPath $installDirectory -Recurse -Force
        }
        if (Test-Path $testerShortcut) {
            Remove-Item -LiteralPath $testerShortcut -Force
        }
        if ($RemoveData -and (Test-Path $dataDirectory)) {
            Remove-Item -LiteralPath $dataDirectory -Recurse -Force
        }
        Write-Host "Cuadra POS Agent desinstalado."
        if (-not $RemoveData) {
            Write-Host "La configuración se conservó en: $dataDirectory"
        }
    }
}

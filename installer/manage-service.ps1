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

switch ($Action) {
    "Install" {
        Assert-Administrator
        $source = (Resolve-Path $ExecutablePath).Path
        New-Item -ItemType Directory -Path $installDirectory -Force | Out-Null
        New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null

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

        & $installedExecutable --install-ca
        if ($LASTEXITCODE -ne 0) {
            throw "No se pudo preparar el certificado local."
        }

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
            "URL=https://localhost:17443/tester"
            "IconFile=$installedExecutable"
            "IconIndex=0"
        ) | Set-Content -LiteralPath $testerShortcut -Encoding ascii
        Start-Service -Name $serviceName
        Wait-ServiceState "Running"
        Write-Host "Cuadra POS Agent instalado y ejecutándose en segundo plano."
        Write-Host "Interfaz: https://localhost:17443/tester"
    }

    "Status" {
        $service = Get-AgentService
        if (-not $service) {
            Write-Host "Cuadra POS Agent no está instalado."
            exit 1
        }
        $service | Select-Object Name, DisplayName, Status, StartType
        try {
            $health = Invoke-RestMethod "https://localhost:17443/health"
            $health | Format-List
        }
        catch {
            Write-Warning "El servicio existe pero el endpoint no respondió: $($_.Exception.Message)"
        }
    }

    "Start" {
        Assert-Administrator
        Start-Service -Name $serviceName
        Wait-ServiceState "Running"
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

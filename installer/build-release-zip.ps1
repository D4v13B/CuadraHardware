param(
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version = "0.1.2",
    [switch]$SkipChecks
)

$ErrorActionPreference = "Stop"
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$dist = [IO.Path]::GetFullPath((Join-Path $projectRoot "dist"))
$packageName = "CuadraPOSAgent-$Version-windows-x64"
$stage = [IO.Path]::GetFullPath((Join-Path $dist $packageName))
$zip = Join-Path $dist "$packageName.zip"

if (-not $stage.StartsWith($dist + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "La carpeta temporal quedó fuera de dist."
}

Push-Location $projectRoot
try {
    if (-not $SkipChecks) {
        cargo fmt --check
        if ($LASTEXITCODE -ne 0) { throw "cargo fmt falló" }
        cargo clippy --release --target x86_64-pc-windows-msvc -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw "cargo clippy falló" }
        cargo test --release --target x86_64-pc-windows-msvc
        if ($LASTEXITCODE -ne 0) { throw "cargo test falló" }
    }

    cargo build --release --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "cargo build falló" }

    New-Item -ItemType Directory -Path $dist -Force | Out-Null
    if (Test-Path -LiteralPath $stage) {
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
    if (Test-Path -LiteralPath $zip) {
        Remove-Item -LiteralPath $zip -Force
    }

    New-Item -ItemType Directory -Path (Join-Path $stage "installer") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $stage "config") -Force | Out-Null

    Copy-Item "target\x86_64-pc-windows-msvc\release\cuadra-pos-agent.exe" $stage
    Copy-Item "installer\manage-service.ps1" (Join-Path $stage "installer")
    Copy-Item "config\config.example.json" (Join-Path $stage "config")
    Copy-Item "installer\package\Install.ps1" (Join-Path $stage "Install.ps1")
    Copy-Item "installer\package\LEEME.txt" (Join-Path $stage "LEEME.txt")

    Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip -CompressionLevel Optimal
    if (-not (Test-Path -LiteralPath $zip)) { throw "No se generó el ZIP" }

    $hash = (Get-FileHash $zip -Algorithm SHA256).Hash
    "$hash  $([IO.Path]::GetFileName($zip))" |
        Set-Content (Join-Path $dist "SHA256SUMS.txt") -Encoding ascii
    Write-Host "ZIP generado: $zip"
    Write-Host "SHA256: $hash"
}
finally {
    Pop-Location
}

param(
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version = "0.1.4",
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $projectRoot
try {
    foreach ($command in @("cargo", "wix")) {
        if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
            throw "No se encontró '$command'. Consulte README.md para instalar los prerrequisitos."
        }
    }
    & (Join-Path $projectRoot "assets\generate-assets.ps1")
    cargo fmt --check
    if ($LASTEXITCODE -ne 0) { throw "cargo fmt falló" }
    cargo clippy --release --target x86_64-pc-windows-msvc -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "cargo clippy falló" }
    if (-not $SkipTests) {
        cargo test --release --target x86_64-pc-windows-msvc
        if ($LASTEXITCODE -ne 0) { throw "cargo test falló" }
    }
    cargo build --release --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "cargo build falló" }

    $dist = Join-Path $projectRoot "dist"
    New-Item -ItemType Directory -Path $dist -Force | Out-Null
    $output = Join-Path $dist "CuadraPOSAgent-$Version-x64.msi"
    wix build ".\installer\Product.wxs" -arch x64 `
        -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
        -d "ProductVersion=$Version" -out $output
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path $output)) { throw "No se generó el MSI" }

    $hash = (Get-FileHash $output -Algorithm SHA256).Hash
    "$hash  $([IO.Path]::GetFileName($output))" | Set-Content (Join-Path $dist "SHA256SUMS.txt") -Encoding ascii
    Write-Host "Instalador generado: $output"
}
finally {
    Pop-Location
}

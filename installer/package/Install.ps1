#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"
$manager = Join-Path $PSScriptRoot "installer\manage-service.ps1"
$executable = Join-Path $PSScriptRoot "cuadra-pos-agent.exe"

if (-not (Test-Path -LiteralPath $manager)) {
    throw "No se encontró installer\manage-service.ps1. Extraiga todo el ZIP antes de instalar."
}
if (-not (Test-Path -LiteralPath $executable)) {
    throw "No se encontró cuadra-pos-agent.exe. Extraiga todo el ZIP antes de instalar."
}

& $manager Install -ExecutablePath $executable

# Instalación y administración del servicio

## Paquete ZIP para clientes

Descargue `CuadraPOSAgent-<versión>-windows-x64.zip` desde la página de releases
de GitHub, extraiga todo su contenido y ejecute `Install.ps1` como administrador.
El paquete contiene el ejecutable, la configuración predeterminada y los scripts
necesarios para registrar el servicio y el certificado local.

## Comportamiento esperado en producción

El agente debe ejecutarse como servicio de Windows. Abrir el `.exe` directamente
lo ejecuta en modo consola y lo hace depender de esa ventana.

El servicio instalado:

- se llama `CuadraPosAgent`;
- utiliza la cuenta `LocalSystem`;
- inicia automáticamente con Windows;
- se reinicia si termina por error;
- no muestra una terminal;
- sirve la interfaz en `https://localhost:17443/tester` y
  `http://localhost:17442/tester`.

## Instalación sin MSI

Primero genere el ejecutable:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

Abra PowerShell como administrador:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\installer\manage-service.ps1 Install
```

La acción `Install`:

1. Copia el ejecutable a `C:\Program Files\Cuadra POS Agent`.
2. Crea o conserva `config.json` en `ProgramData`.
3. Genera certificados únicos para la PC e instala la CA local.
4. Registra el servicio con inicio automático.
5. Configura tres intentos de recuperación.
6. Inicia el servicio.
7. Crea un acceso en el menú Inicio para abrir la interfaz.

## Administración

Todos los comandos salvo `Status` requieren PowerShell como administrador.

```powershell
# Estado del servicio y health check
.\installer\manage-service.ps1 Status

# Iniciar
.\installer\manage-service.ps1 Start

# Detener temporalmente
.\installer\manage-service.ps1 Stop

# Detener y deshabilitar el inicio automático
.\installer\manage-service.ps1 Disable

# Rehabilitar e iniciar
.\installer\manage-service.ps1 Enable

# Desinstalar conservando datos
.\installer\manage-service.ps1 Uninstall

# Desinstalar y borrar configuración, token, certificados y logs
.\installer\manage-service.ps1 Uninstall -RemoveData
```

`Stop` no cambia el tipo de inicio: Windows podrá iniciarlo en el próximo
arranque. Use `Disable` para detenerlo de forma permanente hasta ejecutar
`Enable`.

## Actualización manual

Compile la nueva versión y vuelva a ejecutar:

```powershell
.\installer\manage-service.ps1 Install
```

El script detiene el servicio, reemplaza el binario, conserva los datos y vuelve
a iniciarlo.

## MSI con WiX

Requisitos adicionales:

```powershell
dotnet tool install --global wix
wix extension add WixToolset.UI.wixext
wix extension add WixToolset.Util.wixext
```

Construcción:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\installer\build-installer.ps1 -Version 0.1.3
```

Salida:

```text
dist\CuadraPOSAgent-0.1.3-x64.msi
dist\SHA256SUMS.txt
```

El MSI registra e inicia el mismo servicio mediante `ServiceInstall` y
`ServiceControl`.

## Setup.exe opcional

Con Inno Setup 6 instalado:

```powershell
& "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" `
  "/DMyAppVersion=0.1.3" `
  ".\installer\CuadraPOSAgent.iss"
```

Salida esperada:

```text
dist\CuadraPOSAgent-Setup-0.1.3.exe
```

## Verificación posterior

```powershell
Get-Service CuadraPosAgent
Get-NetTCPConnection -LocalPort 17443 -State Listen
Invoke-RestMethod https://localhost:17443/health
Get-ChildItem "C:\ProgramData\Cuadra ERP\Cuadra POS Agent\logs"
```

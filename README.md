# Cuadra POS Agent

Agente local de Windows que conecta Cuadra POS con impresoras térmicas y gavetas
de dinero mediante Windows Spooler, TCP/IP o puerto serial.

## Estado actual

- API local REST sobre HTTPS en `127.0.0.1:17443` y HTTP en `127.0.0.1:17442`.
- Servicio de Windows `CuadraPosAgent` con inicio automático.
- Interfaz interna de diagnóstico en `https://localhost:17443/tester`.
- Impresión RAW por Windows Spooler, TCP y serial.
- Apertura de gaveta mediante `cash: true` y el pulso ESC/POS
- Corte de papel mediante `cut: true`, tres avances de línea y el comando
  ESC/POS de corte parcial `GS V 1` (`0A 0A 0A 1D 56 01`).
  `ESC p 0 25 250`.
- Enumeración de impresoras de Windows y puertos COM.
- Token de autenticación y certificado únicos por equipo.

> La implementación actual utiliza REST/HTTPS. El protocolo WebSocket descrito en
> propuestas iniciales todavía no forma parte del agente.

## Uso rápido

### Desarrollo

```powershell
cd "C:\Users\dbust\OneDrive\Escritorio\CuadraHardware"
$env:CUADRA_POS_AGENT_DATA_DIR = "$PWD\data"
cargo run -- --console
```

El navegador abrirá automáticamente la consola interna. Para no abrirlo:

```powershell
cargo run -- --console --no-browser
```

### Producción como servicio

Compile el ejecutable y abra PowerShell como administrador:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
Set-ExecutionPolicy -Scope Process Bypass
.\installer\manage-service.ps1 Install
```

Después puede cerrar la terminal. El servicio seguirá ejecutándose en segundo
plano y arrancará con Windows.

```powershell
.\installer\manage-service.ps1 Status
```

### Detener permanentemente

```powershell
.\installer\manage-service.ps1 Disable
```

Para rehabilitarlo:

```powershell
.\installer\manage-service.ps1 Enable
```

## Compilación release

```powershell
rustup target add x86_64-pc-windows-msvc
cargo fmt --check
cargo clippy --release --target x86_64-pc-windows-msvc -- -D warnings
cargo test --release --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

Resultado:

```text
target\x86_64-pc-windows-msvc\release\cuadra-pos-agent.exe
```

## Documentación

- [Índice de documentación](docs/README.md)
- [Instalación y servicio](docs/INSTALLATION.md)
- [Desarrollo y compilación](docs/DEVELOPMENT.md)
- [API REST](docs/API.md)
- [Interfaz interna de pruebas](docs/TESTER.md)
- [Configuración](docs/CONFIGURATION.md)
- [Arquitectura](docs/ARCHITECTURE.md)
- [Seguridad](docs/SECURITY.md)
- [Solución de problemas](docs/TROUBLESHOOTING.md)

## Rutas de producción

```text
C:\Program Files\Cuadra POS Agent\
    cuadra-pos-agent.exe

C:\ProgramData\Cuadra ERP\Cuadra POS Agent\
    config.json
    agent-token
    certs\
    logs\
```

No almacene tokens ni claves privadas en el frontend o en variables `VITE_*`.

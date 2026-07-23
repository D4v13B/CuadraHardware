# Desarrollo y compilación

## Prerrequisitos

- Rust estable.
- Target `x86_64-pc-windows-msvc`.
- Visual Studio 2022 Build Tools con desarrollo de escritorio C++, MSVC x64 y
  Windows SDK.

Instale el target una sola vez:

```powershell
rustup target add x86_64-pc-windows-msvc
```

## Ejecución local

Desde la raíz del proyecto:

```powershell
$env:CUADRA_POS_AGENT_DATA_DIR = "$PWD\data"
cargo run -- --console
```

La variable evita escribir datos de desarrollo en `ProgramData`. Al iniciar, el
agente crea:

```text
data\config.json
data\agent-token
data\certs\root-ca.crt
data\certs\localhost.crt
data\certs\localhost.key
data\logs\
```

El navegador abre `https://localhost:17443/tester`. Para evitarlo:

```powershell
cargo run -- --console --no-browser
```

## Certificado local

Para confiar en el certificado de desarrollo, abra PowerShell como administrador
manteniendo la misma variable de datos:

```powershell
$env:CUADRA_POS_AGENT_DATA_DIR = "$PWD\data"
cargo run -- --install-ca
```

Sin instalar la CA puede verificar con:

```powershell
curl.exe -k https://127.0.0.1:17443/health
```

## Modos del ejecutable

| Argumento | Uso |
| --- | --- |
| Sin argumentos o `--console` | Servidor en primer plano y apertura del navegador |
| `--console --no-browser` | Servidor en primer plano sin navegador |
| `--service` | Entrada utilizada exclusivamente por Windows Service Control Manager |
| `--install-ca` | Genera certificados si faltan e instala la CA local |

No ejecute `--service` manualmente: el dispatcher espera ser iniciado por
Windows.

## Validación

```powershell
cargo fmt --check
cargo clippy --release --target x86_64-pc-windows-msvc -- -D warnings
cargo test --release --target x86_64-pc-windows-msvc
```

Las pruebas actuales verifican la comparación del token y que el comando de
gaveta sólo se agregue cuando `cash` sea verdadero.

## Compilación release

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

El target MSVC usa `target-feature=+crt-static` desde `.cargo/config.toml`. El
ejecutable resultante incluye el runtime de Visual C++ y no requiere que la PC
cliente tenga `VCRUNTIME140.dll` instalado.

Ejecutable:

```text
target\x86_64-pc-windows-msvc\release\cuadra-pos-agent.exe
```

Hash de distribución:

```powershell
Get-FileHash `
  ".\target\x86_64-pc-windows-msvc\release\cuadra-pos-agent.exe" `
  -Algorithm SHA256
```

## Recursos visuales

Para regenerar icono y bitmaps del instalador:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\assets\generate-assets.ps1
```

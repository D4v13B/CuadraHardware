# API REST

## Base URL

```text
https://localhost:17443
http://localhost:17442
```

Ambas URLs sirven la misma API simultáneamente. El agente escucha únicamente en
la dirección loopback configurada y no debe exponerse directamente en la red
local.

## Autenticación

Los endpoints operativos requieren:

```http
Authorization: Bearer <agent-token>
```

El token se encuentra en:

```text
C:\ProgramData\Cuadra ERP\Cuadra POS Agent\agent-token
```

Durante desarrollo, si se define `CUADRA_POS_AGENT_DATA_DIR`, se encuentra en ese
directorio. La interfaz interna servida por el agente utiliza un flujo del mismo
origen y no solicita pegar el token.

El POS puede obtener el token automáticamente desde la misma computadora:

```http
GET https://localhost:17443/v1/token
```

```json
{"token":"..."}
```

El endpoint valida que la conexión TCP provenga de loopback (`127.0.0.1` o
`::1`) y envía `Cache-Control: no-store`. El agente no escucha en interfaces de
red, por lo que otra computadora no puede solicitarlo directamente.

## Endpoints

| Método | Ruta | Autenticación | Descripción |
| --- | --- | --- | --- |
| `GET` | `/` | No | Redirige funcionalmente a la interfaz embebida |
| `GET` | `/tester` | No | Consola interna de diagnóstico |
| `GET` | `/health` | No | Estado y versión del agente |
| `GET` | `/v1/token` | Local | Obtiene automáticamente el token de esta computadora |
| `GET` | `/v1/printers` | Sí | Impresoras de Windows y puertos seriales |
| `POST` | `/v1/print` | Sí | Envía bytes RAW y opcionalmente abre la gaveta o corta el papel |

Las llamadas realizadas desde `/tester` son autorizadas como mismo origen local.

## `GET /health`

Respuesta:

```json
{
  "status": "ok",
  "service": "cuadra-pos-agent",
  "version": "0.1.0"
}
```

Ejemplo:

```powershell
Invoke-RestMethod https://localhost:17443/health
```

## `GET /v1/printers`

Ejemplo:

```powershell
$token = (Get-Content `
  "C:\ProgramData\Cuadra ERP\Cuadra POS Agent\agent-token" -Raw).Trim()

Invoke-RestMethod https://localhost:17443/v1/printers `
  -Headers @{ Authorization = "Bearer $token" }
```

Respuesta:

```json
{
  "windowsPrinters": [
    {
      "id": "M4202",
      "name": "M4202",
      "driver": "M4202 Driver",
      "port": "USB004",
      "status": "ready",
      "statusCode": 0,
      "isDefault": true,
      "connectionType": "windows"
    }
  ],
  "serialPorts": [
    {
      "portName": "COM3",
      "displayName": "Puerto serial COM3",
      "vendorId": null,
      "productId": null,
      "serialNumber": null
    }
  ]
}
```

Estados normalizados de impresora: `ready`, `printing`, `offline` y `error`.
`statusCode` conserva el valor original del Spooler.

## `POST /v1/print`

`dataBase64` contiene los bytes RAW que recibirá el dispositivo. El máximo por
trabajo es 8 MiB antes de añadir los comandos opcionales de gaveta y corte.

### Windows Spooler

```json
{
  "transport": "windowsSpooler",
  "printer": "M4202",
  "dataBase64": "Q1VBRFJBIFBPUwoKCg==",
  "cash": false,
  "cut": true
}
```

### Red TCP

```json
{
  "transport": "network",
  "host": "192.168.1.50",
  "port": 9100,
  "dataBase64": "Q1VBRFJBIFBPUwoKCg==",
  "cash": false,
  "cut": true
}
```

La conexión tiene un límite de 5 segundos y la escritura uno de 15 segundos.

### Puerto serial

```json
{
  "transport": "serial",
  "port": "COM3",
  "baudRate": 9600,
  "dataBase64": "Q1VBRFJBIFBPUwoKCg==",
  "cash": false,
  "cut": true
}
```

### Gaveta de dinero

Para abrirla después de enviar el contenido:

```json
{
  "transport": "windowsSpooler",
  "printer": "M4202",
  "dataBase64": "G0A=",
  "cash": true
}
```

Cuando `cash` es `true`, el agente añade estos cinco bytes:

```text
1B 70 00 19 FA
ESC p 0 25 250
```

Si `cash` se omite o es `false`, no se agrega ningún comando de gaveta. El agente
no intenta interpretar el tipo de pago; esa decisión pertenece al POS.

### Corte de papel

Para cortar el papel al final del trabajo, envíe `"cut": true`. El agente añade
el comando de corte completo ESC/POS:

```text
1D 56 00
GS V 0
```

Si `cut` se omite o es `false`, no se agrega el comando. Cuando `cash` y `cut`
son verdaderos, el pulso de gaveta se envía primero y el corte al final.

Respuesta exitosa:

```json
{
  "accepted": true,
  "bytesWritten": 128,
  "cashDrawerOpenRequested": true,
  "paperCutRequested": true
}
```

`cashDrawerOpenRequested` confirma que se añadió el pulso al trabajo, no que
exista un sensor capaz de confirmar físicamente que la gaveta abrió.
`paperCutRequested` confirma que se añadió el comando, no que la impresora tenga
cortador o pueda confirmar físicamente el corte.

## Ejemplo PowerShell completo

```powershell
$token = (Get-Content `
  "C:\ProgramData\Cuadra ERP\Cuadra POS Agent\agent-token" -Raw).Trim()
$headers = @{ Authorization = "Bearer $token" }
$bytes = [Text.Encoding]::UTF8.GetBytes("CUADRA POS`nPRUEBA`n`n`n")
$body = @{
  transport = "windowsSpooler"
  printer = "M4202"
  dataBase64 = [Convert]::ToBase64String($bytes)
  cash = $true
  cut = $true
} | ConvertTo-Json

Invoke-RestMethod https://localhost:17443/v1/print `
  -Method Post -Headers $headers -ContentType "application/json" -Body $body
```

## Errores HTTP

| Estado | Significado |
| --- | --- |
| `400` | JSON, Base64, dispositivo o trabajo inválido |
| `401` | Token ausente o incorrecto |
| `403` | Origen del navegador no permitido |
| `404` | Ruta inexistente |

Los errores de dispositivo se devuelven como texto descriptivo.

## CORS

Con `"*"` en `security.allowedOrigins`, cualquier origen del navegador puede
utilizar la API. El agente refleja el origen recibido y sigue exigiendo el token
Bearer. Las llamadas de servidor a servidor normalmente no incluyen `Origin`,
pero también necesitan el token.

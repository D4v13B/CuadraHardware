# Configuración

## Ubicación

Producción:

```text
C:\ProgramData\Cuadra ERP\Cuadra POS Agent\config.json
```

Desarrollo:

```powershell
$env:CUADRA_POS_AGENT_DATA_DIR = "$PWD\data"
```

La variable cambia el directorio completo de datos, incluyendo configuración,
token, certificados y logs.

## Configuración predeterminada

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 17443,
    "tlsEnabled": true,
    "httpPort": 17442
  },
  "security": {
    "allowedOrigins": [
      "*"
    ],
    "requireAuthentication": true
  },
  "logging": {
    "level": "info",
    "directory": "C:\\ProgramData\\Cuadra ERP\\Cuadra POS Agent\\logs"
  }
}
```

## Propiedades

### `server.host`

Debe ser una dirección loopback. El agente rechaza configuraciones que intenten
escuchar en una interfaz de red.

### `server.port`

Puerto HTTPS local. Si se cambia, actualice el cliente POS y use la nueva URL en
la interfaz de pruebas.

### `server.tlsEnabled`

Cuando es `true`, `server.port` sirve HTTPS y `server.httpPort` sirve HTTP al
mismo tiempo. Si es `false`, `server.port` sirve únicamente HTTP.

### `server.httpPort`

Puerto HTTP local adicional; el valor predeterminado es `17442`, incluso para
archivos de configuración antiguos que no incluyan esta propiedad. Use `null`
para desactivar el listener HTTP cuando `tlsEnabled` sea `true`. Debe ser
diferente de `server.port`.

### `security.allowedOrigins`

Use `"*"` para aceptar solicitudes CORS desde cualquier origen:

```json
"*"
```

El agente refleja el origen recibido en `Access-Control-Allow-Origin`. La
autenticación Bearer continúa siendo obligatoria aunque CORS permita el origen.
También puede reemplazar `"*"` por una lista de orígenes exactos para restringir
el acceso desde navegadores.

### `security.requireAuthentication`

Cuando es `true`, los endpoints operativos exigen `Bearer token`. Debe mantenerse
habilitado en producción.

### `logging.level`

Filtro compatible con `tracing-subscriber`, por ejemplo `error`, `warn`, `info`,
`debug` o `cuadra_pos_agent=debug`.

### `logging.directory`

Directorio de logs rotados diariamente.

## Aplicar cambios

Después de editar `config.json`, reinicie el servicio:

```powershell
Restart-Service CuadraPosAgent
```

o:

```powershell
.\installer\manage-service.ps1 Stop
.\installer\manage-service.ps1 Start
```

## Configuración física por caja

La versión actual recibe la configuración del dispositivo en cada solicitud de
`/v1/print`. Todavía no guarda perfiles de impresora por `terminal_id`. Esa capa
puede residir en el POS o añadirse al agente en una versión posterior.

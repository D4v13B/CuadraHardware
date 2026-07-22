# Seguridad

## Superficie de red

El agente exige que `server.host` sea loopback y por defecto escucha en:

```text
127.0.0.1:17443
```

No abra este puerto en el firewall ni cambie la dirección a `0.0.0.0`.

## TLS por equipo

En el primer arranque se generan:

```text
certs\root-ca.crt
certs\localhost.crt
certs\localhost.key
```

La CA y la clave privada son únicas para cada instalación. El certificado incluye
`localhost`, `127.0.0.1` y `::1` como nombres alternativos.

`--install-ca` instala únicamente la CA pública en el almacén raíz del equipo. La
clave privada permanece en `ProgramData`.

## Token

El agente genera 48 bytes aleatorios y los codifica en Base64 URL-safe. El token
se almacena en `agent-token` con herencia de ACL deshabilitada.

Los endpoints operativos validan:

```http
Authorization: Bearer <token>
```

La comparación tiene tiempo constante para valores de igual longitud.

## Frontend

No incluya el token en:

- variables `VITE_*`;
- repositorios;
- archivos JavaScript estáticos;
- parámetros de URL;
- logs o herramientas analíticas.

Los valores `VITE_*` se incorporan al frontend compilado. El POS debe obtener el
token mediante un proceso de emparejamiento o configuración segura.

## CORS y consola interna

El valor `"*"` en `security.allowedOrigins` permite cualquier origen CORS. La
interfaz `/tester` es servida por el mismo agente y utiliza el mismo origen local.
La autenticación Bearer permanece activa para las solicitudes externas.

CORS protege el contexto del navegador, pero no sustituye la autenticación para
programas locales. Los clientes que llaman directamente a la API deben usar el
token.

`GET /v1/token` permite que el POS obtenga el token sin configuración manual,
pero sólo acepta conexiones TCP desde loopback. Como CORS está configurado con
`"*"`, cualquier página ejecutada en el navegador de esa misma computadora puede
solicitarlo; la restricción evita otras computadoras, no otros sitios abiertos
localmente. La respuesta se marca como `no-store`.

## Permisos

Producción utiliza `LocalSystem`. Token, claves y directorio de certificados se
restringen a `SYSTEM`, administradores y la identidad que los genera. Los logs no
deben contener el token ni el contenido completo de recibos.

## Firma de código

Antes de distribuir:

1. compile el EXE;
2. firme el EXE;
3. cree el MSI o Setup.exe;
4. firme el instalador;
5. utilice sello de tiempo SHA-256.

Ejemplo:

```powershell
signtool.exe sign /fd SHA256 /td SHA256 `
  /tr http://timestamp.digicert.com `
  /f ".\certificates\code-signing.pfx" `
  /p $env:SIGNING_CERT_PASSWORD `
  ".\target\x86_64-pc-windows-msvc\release\cuadra-pos-agent.exe"
```

No almacene el PFX ni su contraseña en el repositorio.

## Recomendaciones de producción

- Mantenga `tlsEnabled` y `requireAuthentication` en `true`.
- Autorice únicamente dominios HTTPS controlados.
- Firme binarios e instaladores.
- Rote el token si se sospecha exposición.
- Revise permisos después de migrar manualmente `ProgramData`.
- No distribuya certificados o claves de una PC a otra.

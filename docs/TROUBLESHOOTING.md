# Solución de problemas

## El `.exe` abre una terminal y se cierra

El ejecutable no es un programa de escritorio tradicional. Sin instalación se
ejecuta en modo consola. Para ver el error:

```powershell
.\target\x86_64-pc-windows-msvc\release\cuadra-pos-agent.exe --console
```

Para producción, instálelo como servicio y abra la interfaz desde el menú Inicio:

```powershell
.\installer\manage-service.ps1 Install
```

## Error `10048`: puerto ocupado

Otra instancia ya escucha en `17443`:

```powershell
Get-NetTCPConnection -LocalPort 17443 -State Listen
```

Compruebe primero si el servicio ya funciona:

```powershell
Get-Service CuadraPosAgent
Invoke-RestMethod https://localhost:17443/health
```

No inicie una segunda copia si el servicio está activo.

## El servicio no está instalado

```powershell
Get-Service CuadraPosAgent
```

Si devuelve que no existe, abra PowerShell como administrador:

```powershell
.\installer\manage-service.ps1 Install
```

## El servicio se detiene

Revise:

```powershell
Get-Service CuadraPosAgent
Get-Content "C:\ProgramData\Cuadra ERP\Cuadra POS Agent\logs\agent.log*" -Tail 100
sc.exe queryex CuadraPosAgent
```

Confirme que `config.json` es JSON válido y que el puerto está libre.

## Detenerlo definitivamente

`Stop` es temporal porque el servicio conserva inicio automático. Use:

```powershell
.\installer\manage-service.ps1 Disable
```

Para reactivarlo:

```powershell
.\installer\manage-service.ps1 Enable
```

## Advertencia de certificado

Abra PowerShell como administrador:

```powershell
& "C:\Program Files\Cuadra POS Agent\cuadra-pos-agent.exe" --install-ca
```

Cierre y vuelva a abrir el navegador. Confirme que la fecha y hora de Windows son
correctas.

## `401 credenciales inválidas`

Lea el token sin espacios adicionales:

```powershell
$token = (Get-Content `
  "C:\ProgramData\Cuadra ERP\Cuadra POS Agent\agent-token" -Raw).Trim()
```

Envíelo como `Bearer`. La interfaz `/tester` gestiona su acceso interno y no
requiere pegarlo.

## `403 origen no permitido`

Confirme que `security.allowedOrigins` permita cualquier origen:

```json
"*"
```

Reinicie el servicio después del cambio.

## No aparecen impresoras

1. Compruebe que la impresora aparece en Configuración de Windows.
2. Reinicie el servicio `Spooler` de Windows si está bloqueado.
3. Pulse **Actualizar dispositivos** en `/tester`.
4. Verifique que el driver sea x64 y pueda imprimir una página de prueba.

Para serial, compruebe en el Administrador de dispositivos si aparece `COM3`,
`COM4`, etc., y cierre programas que mantengan el puerto abierto.

## La impresión de Windows no sale

- Seleccione el nombre exacto devuelto por `/v1/printers`.
- Compruebe que el driver acepte trabajos RAW.
- Vacíe trabajos atascados en la cola.
- Pruebe primero sin comandos específicos de marca.

## La impresora TCP no responde

```powershell
Test-NetConnection 192.168.1.50 -Port 9100
```

Revise IP, VLAN, firewall y puerto configurado. El agente agota la conexión tras
5 segundos.

## La gaveta no abre

1. Confirme que está conectada a la impresora correcta.
2. Use **Probar gaveta** en `/tester`.
3. Verifique que la respuesta indique
   `cashDrawerOpenRequested: true`.
4. Confirme compatibilidad ESC/POS y pin de gaveta.
5. Pruebe el driver RAW; algunos drivers gráficos filtran comandos ESC/POS.

El comando actual es `ESC p 0 25 250`. Algunos modelos requieren pin 1 u otros
tiempos; esos valores todavía no son configurables.

## Logs

```powershell
Get-ChildItem "C:\ProgramData\Cuadra ERP\Cuadra POS Agent\logs"
Get-Content "C:\ProgramData\Cuadra ERP\Cuadra POS Agent\logs\agent.log*" -Tail 100
```

Para más detalle cambie temporalmente `logging.level` a `debug` y reinicie el
servicio. Evite compartir logs sin revisar datos sensibles.

# Arquitectura

## Flujo general

```text
Cuadra POS / Interfaz de pruebas
              |
          HTTPS REST
              |
      Cuadra POS Agent
       /      |       \
 Spooler   Serial    TCP/IP
    |         |         |
 Impresora térmica / dispositivo ESC/POS
                       |
                 Gaveta de dinero
```

El navegador nunca accede directamente al Spooler, COM o socket de la impresora.
Todas las operaciones pasan por el agente local.

## Módulos

| Archivo | Responsabilidad |
| --- | --- |
| `src/main.rs` | Selección de consola, servicio, instalación de CA y navegador |
| `src/service.rs` | Integración con Windows Service Control Manager |
| `src/server.rs` | Router Axum, TLS, CORS, autenticación e interfaz embebida |
| `src/config.rs` | Modelo, valores predeterminados y carga de `config.json` |
| `src/security.rs` | Token, CA, certificado localhost y ACL de secretos |
| `src/printer/mod.rs` | Contrato de impresión, Base64, límite, gaveta y corte |
| `src/printer/windows_spooler.rs` | Enumeración e impresión RAW de Windows |
| `src/printer/serial.rs` | Enumeración y escritura en puertos seriales |
| `src/printer/network.rs` | Escritura TCP con límites de tiempo |
| `assets/tester.html` | Interfaz interna embebida dentro del ejecutable |

## Ciclo del servicio

1. Windows inicia `cuadra-pos-agent.exe --service`.
2. El proceso registra su manejador de controles.
3. Carga o crea configuración, token y certificados.
4. Inicia HTTPS en loopback.
5. Reporta estado `Running`.
6. Al recibir `Stop` o `Shutdown`, cancela el servidor.
7. Espera el cierre controlado y reporta `Stopped`.

Si el servidor termina durante el arranque, el servicio también finaliza para que
Windows aplique la política de recuperación.

## Flujo de impresión

1. El cliente convierte el contenido ESC/POS a bytes.
2. Codifica los bytes en Base64.
3. Envía `/v1/print` con el transporte y dispositivo.
4. El agente valida autenticación, origen, Base64 y tamaño.
5. Si `cash` es verdadero, agrega el pulso de gaveta.
6. Si `cut` es verdadero, agrega el comando de corte al final.
7. Escribe el trabajo completo al Spooler, puerto serial o socket TCP.
8. Devuelve los bytes escritos y si se solicitaron la gaveta y el corte.

## Datos estáticos y modificables

```text
Program Files
  Ejecutable instalado

ProgramData
  Configuración, token, certificados y logs
```

Los archivos que cambian durante la operación nunca se guardan dentro de
`Program Files`.

## Decisiones actuales

- Axum y Tokio para servidor y concurrencia.
- Rustls para HTTPS.
- `windows-service` para el ciclo del servicio.
- `windows-sys` para Spooler RAW y enumeración.
- HTML/CSS/JavaScript embebido, sin runtime Node en producción.
- Un trabajo se entrega completo a un solo transporte.

## Límites conocidos

- No existe todavía protocolo WebSocket.
- No se guardan perfiles por caja o `terminal_id`.
- No hay confirmación física del estado de la gaveta.
- El pulso usa pin 0 y tiempos 25/250 fijos.
- `/v1/printers` enumera Spooler y serial; las impresoras TCP se configuran
  manualmente por host y puerto.
- No hay cola persistente propia; Windows Spooler sí conserva los trabajos que
  recibe por ese transporte.

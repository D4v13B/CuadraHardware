# Documentación de Cuadra POS Agent

| Documento | Contenido |
| --- | --- |
| [Instalación](INSTALLATION.md) | Instalación como servicio, MSI, Setup.exe y administración |
| [Desarrollo](DEVELOPMENT.md) | Prerrequisitos, ejecución local, validaciones y compilación |
| [API REST](API.md) | Endpoints, autenticación, modelos y ejemplos |
| [Interfaz de pruebas](TESTER.md) | Uso de la consola interna para impresora y gaveta |
| [Configuración](CONFIGURATION.md) | `config.json`, rutas y orígenes permitidos |
| [Arquitectura](ARCHITECTURE.md) | Componentes, flujo de impresión y ciclo del servicio |
| [Seguridad](SECURITY.md) | TLS, certificados, token, CORS y permisos |
| [Solución de problemas](TROUBLESHOOTING.md) | Diagnóstico de servicio, puerto, TLS, impresora y gaveta |

## Convenciones

- Los comandos están escritos para PowerShell 7 o Windows PowerShell.
- Los comandos que modifican servicios, `Program Files` o certificados requieren
  una terminal ejecutada como administrador.
- `cash: true` significa “abrir gaveta”; `cash: false` o la ausencia del campo
  significa “no abrir”.
- La API actual es REST/HTTPS. No existe aún un endpoint WebSocket.

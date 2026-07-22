# Interfaz interna de pruebas

## Acceso

Con el agente ejecutándose:

```text
https://localhost:17443/tester
```

También está disponible sin TLS en:

```text
http://localhost:17442/tester
```

La instalación manual crea además `Cuadra POS Agent` en el menú Inicio. Ese
acceso abre la página, no otra instancia del servidor.

## Funciones

La interfaz permite:

1. comprobar `/health` y mostrar la versión;
2. actualizar impresoras del Windows Spooler;
3. actualizar puertos seriales;
4. configurar IP y puerto de una impresora de red;
5. enviar texto de prueba;
6. enviar únicamente el pulso de gaveta;
7. inspeccionar la solicitud y respuesta de la API.

## Probar impresión

1. Seleccione `Impresora instalada en Windows`, `USB / Puerto serial` o
   `Impresora de red`.
2. Seleccione o complete el dispositivo.
3. Modifique el contenido de prueba si es necesario.
4. Pulse **Probar impresión**.

Esta acción utiliza `cash: false`; no debe abrir la gaveta.

## Probar gaveta

Seleccione el dispositivo conectado físicamente a la gaveta y pulse
**Probar gaveta**. La solicitud envía un trabajo mínimo seguido de:

```text
ESC p 0 25 250
```

La mayoría de las gavetas se conectan al puerto RJ11/RJ12 de la impresora y no a
la PC. Por eso debe seleccionarse la impresora que controla la gaveta.

La interfaz puede confirmar que el comando fue enviado, pero no puede detectar
si el mecanismo abrió físicamente salvo que el hardware exponga un sensor y un
protocolo adicional.

## Uso con el servicio

No abra el ejecutable instalado para acceder a la interfaz. Compruebe primero:

```powershell
.\installer\manage-service.ps1 Status
```

Luego use el acceso del menú Inicio o escriba la URL en el navegador.

## Advertencia de certificado

Si aparece una advertencia, la CA no está instalada para esa ubicación de datos.
En una terminal elevada:

```powershell
"C:\Program Files\Cuadra POS Agent\cuadra-pos-agent.exe" --install-ca
```

Reinicie el navegador después de instalarla.

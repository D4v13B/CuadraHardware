#define MyAppName "Cuadra POS Agent"
#ifndef MyAppVersion
  #define MyAppVersion "0.1.2"
#endif
#define MyAppPublisher "Cuadra ERP"
#define MyAppExeName "cuadra-pos-agent.exe"

[Setup]
AppId={{2D23BB1A-BC91-49CE-923E-79D58C842821}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL=https://cuadraerp.com
AppSupportURL=https://cuadraerp.com/soporte
DefaultDirName={autopf64}\Cuadra POS Agent
DefaultGroupName=Cuadra POS Agent
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=CuadraPOSAgent-Setup-{#MyAppVersion}
SetupIconFile=..\assets\app.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\assets\license.rtf
WizardImageFile=..\assets\dialog.bmp
WizardSmallImageFile=..\assets\banner.bmp
VersionInfoVersion={#MyAppVersion}
CloseApplications=yes
RestartApplications=no

[Dirs]
Name: "{commonappdata}\Cuadra ERP\Cuadra POS Agent"
Name: "{commonappdata}\Cuadra ERP\Cuadra POS Agent\logs"
Name: "{commonappdata}\Cuadra ERP\Cuadra POS Agent\certs"

[Files]
Source: "..\target\x86_64-pc-windows-msvc\release\cuadra-pos-agent.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\config\config.example.json"; DestDir: "{commonappdata}\Cuadra ERP\Cuadra POS Agent"; DestName: "config.json"; Flags: onlyifdoesntexist uninsneveruninstall

[Run]
Filename: "{sys}\sc.exe"; Parameters: "stop CuadraPosAgent"; Flags: runhidden waituntilterminated; StatusMsg: "Deteniendo versión anterior..."
Filename: "{sys}\sc.exe"; Parameters: "delete CuadraPosAgent"; Flags: runhidden waituntilterminated
Filename: "{app}\{#MyAppExeName}"; Parameters: "--install-ca"; Flags: runhidden waituntilterminated ignoreerrors; StatusMsg: "Preparando conexión segura..."
Filename: "{sys}\sc.exe"; Parameters: "create CuadraPosAgent binPath= ""{app}\{#MyAppExeName} --service"" start= auto DisplayName= ""Cuadra POS Agent"""; Flags: runhidden waituntilterminated; StatusMsg: "Registrando el servicio..."
Filename: "{sys}\sc.exe"; Parameters: "description CuadraPosAgent ""Agente local para hardware de Cuadra POS."""; Flags: runhidden waituntilterminated
Filename: "{sys}\sc.exe"; Parameters: "failure CuadraPosAgent reset= 86400 actions= restart/5000/restart/15000/restart/30000"; Flags: runhidden waituntilterminated
Filename: "{sys}\sc.exe"; Parameters: "start CuadraPosAgent"; Flags: runhidden waituntilterminated; StatusMsg: "Iniciando Cuadra POS Agent..."

[UninstallRun]
Filename: "{sys}\sc.exe"; Parameters: "stop CuadraPosAgent"; Flags: runhidden waituntilterminated; RunOnceId: "StopService"
Filename: "{sys}\sc.exe"; Parameters: "delete CuadraPosAgent"; Flags: runhidden waituntilterminated; RunOnceId: "DeleteService"

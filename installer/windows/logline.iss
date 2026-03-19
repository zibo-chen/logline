; Logline Windows Installer Script (Inno Setup)
; This script creates a Windows installer that:
;   - Installs Logline to Program Files
;   - Creates Start Menu and Desktop shortcuts
;   - Registers .log file association
;   - Provides an uninstaller

#ifndef AppVersion
  #define AppVersion "1.4.2"
#endif

[Setup]
AppId={{B8F2A1E0-5C3D-4A7B-9E8F-1D2C3B4A5E6F}
AppName=Logline
AppVersion={#AppVersion}
AppVerName=Logline {#AppVersion}
AppPublisher=Zibo Chen
AppPublisherURL=https://github.com/zibo-chen/logline
AppSupportURL=https://github.com/zibo-chen/logline/issues
AppUpdatesURL=https://github.com/zibo-chen/logline/releases
DefaultDirName={autopf}\Logline
DefaultGroupName=Logline
AllowNoIcons=yes
LicenseFile=..\..\LICENSE
OutputDir=..\..\target\installer
OutputBaseFilename=logline-{#AppVersion}-windows-x86_64-setup
SetupIconFile=..\..\res\icon.ico
UninstallDisplayIcon={app}\logline.exe
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
PrivilegesRequired=admin
ChangesAssociations=yes
MinVersion=10.0

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "fileassoc"; Description: "Associate .log files with Logline"; GroupDescription: "File Associations:"; Flags: checkedonce

[Files]
Source: "..\..\target\release\logline.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Logline"; Filename: "{app}\logline.exe"
Name: "{group}\{cm:UninstallProgram,Logline}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Logline"; Filename: "{app}\logline.exe"; Tasks: desktopicon

[Registry]
; Register .log file association
Root: HKA; Subkey: "Software\Classes\.log\OpenWithProgids"; ValueType: string; ValueName: "Logline.LogFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\Logline.LogFile"; ValueType: string; ValueName: ""; ValueData: "Log File"; Flags: uninsdeletekey; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\Logline.LogFile\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\logline.exe,0"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\Logline.LogFile\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\logline.exe"" ""%1"""; Tasks: fileassoc

; Register in "Open with" context menu for all files
Root: HKA; Subkey: "Software\Classes\*\shell\Open with Logline"; ValueType: string; ValueName: ""; ValueData: "Open with Logline"; Flags: uninsdeletekey; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\*\shell\Open with Logline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\logline.exe,0"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\*\shell\Open with Logline\command"; ValueType: string; ValueName: ""; ValueData: """{app}\logline.exe"" ""%1"""; Tasks: fileassoc

; Register application capabilities for Default Programs
Root: HKA; Subkey: "Software\Logline\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "Logline"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Logline\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "High-performance real-time log viewer"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Logline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".log"; ValueData: "Logline.LogFile"; Tasks: fileassoc
Root: HKA; Subkey: "Software\Logline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".txt"; ValueData: "Logline.LogFile"; Tasks: fileassoc
Root: HKA; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "Logline"; ValueData: "Software\Logline\Capabilities"; Flags: uninsdeletevalue; Tasks: fileassoc

[Run]
Filename: "{app}\logline.exe"; Description: "{cm:LaunchProgram,Logline}"; Flags: nowait postinstall skipifsilent

; Inno Setup Script for SvagView
#define MyAppName "SvagView"
#define MyAppVersion "0.1.1"
#define MyAppPublisher "SvagView Contributors"
#define MyAppURL "https://github.com/vaibhav-upadhyay/SvagView"
#define MyAppExeName "svagview.exe"

[Setup]
AppId={{D37D328C-28D7-4B29-B101-9F3C6D683A14}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
SourceDir=..
OutputDir=dist
OutputBaseFilename=svagview-v0.1.1-windows-setup
SetupIconFile=resources\SvagView_Logo_Icon.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "associate_svg"; Description: "Associate with SVG files (.svg)"; GroupDescription: "File associations:"; Flags: unchecked
Name: "associate_svgz"; Description: "Associate with compressed SVG files (.svgz)"; GroupDescription: "File associations:"; Flags: unchecked

[Files]
Source: "target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "resources\SvagView_Logo_Icon.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; Associate .svg
Root: HKA; Subkey: "Software\Classes\.svg\OpenWithProgids"; ValueType: string; ValueName: "SvagView.SVG"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate_svg
Root: HKA; Subkey: "Software\Classes\SvagView.SVG"; ValueType: string; ValueName: ""; ValueData: "Scalable Vector Graphics"; Flags: uninsdeletekey; Tasks: associate_svg
Root: HKA; Subkey: "Software\Classes\SvagView.SVG\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Tasks: associate_svg
Root: HKA; Subkey: "Software\Classes\SvagView.SVG\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: associate_svg

; Associate .svgz
Root: HKA; Subkey: "Software\Classes\.svgz\OpenWithProgids"; ValueType: string; ValueName: "SvagView.SVGZ"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate_svgz
Root: HKA; Subkey: "Software\Classes\SvagView.SVGZ"; ValueType: string; ValueName: ""; ValueData: "Compressed Scalable Vector Graphics"; Flags: uninsdeletekey; Tasks: associate_svgz
Root: HKA; Subkey: "Software\Classes\SvagView.SVGZ\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Tasks: associate_svgz
Root: HKA; Subkey: "Software\Classes\SvagView.SVGZ\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: associate_svgz

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

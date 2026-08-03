[Setup]
AppName=ExplorerTags
AppVersion=1.0
AppPublisher=Atabek
DefaultDirName={userappdata}\ExplorerTags
DefaultGroupName=ExplorerTags
OutputDir=InstallerOutput
OutputBaseFilename=ExplorerTags_Setup_v1.0
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=lowest

[Files]
; 1. Наша C++ DLL (собираем из Release)
Source: "build\Release\ExplorerTags.dll"; DestDir: "{app}"; Flags: ignoreversion

; 2. Схема свойств (лежит в папке res/)
Source: "res\PropertySchema.propdesc"; DestDir: "{app}"; Flags: ignoreversion

; 3. C# UI приложение и его библиотеки
Source: "ExplorerTagsUI\bin\Release\net8.0-windows\win-x64\publish\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs

; 4. Папка с иконками (Флаги исправлены - объединили в одну строку)
Source: "Icons\*"; DestDir: "{app}\Icons"; Flags: ignoreversion recursesubdirs skipifsourcedoesntexist

[Registry]
Root: HKCU; Subkey: "Software\Classes\CLSID\{{A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}"; ValueType: string; ValueData: "ExplorerTags Context Menu"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\CLSID\{{A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}\InProcServer32"; ValueType: string; ValueData: "{app}\ExplorerTags.dll"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\CLSID\{{A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}\InProcServer32"; ValueName: "ThreadingModel"; ValueType: string; ValueData: "Apartment"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\shell\ExplorerTags"; ValueName: "ExplorerCommandHandler"; ValueType: string; ValueData: "{{A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}"; Flags: uninsdeletekey

[Run]
Filename: "regsvr32.exe"; Parameters: "/s ""{app}\ExplorerTags.dll"""; Flags: runhidden

[UninstallRun]
Filename: "regsvr32.exe"; Parameters: "/u /s ""{app}\ExplorerTags.dll"""; Flags: runhidden

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
begin
  if CurStep = ssPostInstall then
  begin
    Exec('taskkill.exe', '/f /im dllhost.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec('taskkill.exe', '/f /im explorer.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec('explorer.exe', '', '', SW_SHOWNORMAL, ewNoWait, ResultCode);
  end;
end;
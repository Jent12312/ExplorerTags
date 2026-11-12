[Setup]
AppName=ExplorerTags
AppVersion=2.0
AppPublisher=Atabek
DefaultDirName={userappdata}\ExplorerTags
DefaultGroupName=ExplorerTags
OutputDir=InstallerOutput
OutputBaseFilename=ExplorerTags_Setup_v2.0
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest

[Files]
; 1. Наша Rust DLL (сверхлегкая COM DLL)
Source: "target\release\ExplorerTags.dll"; DestDir: "{app}"; Flags: ignoreversion

; 2. Наше Rust Slint UI приложение (единый автономный exe диалога папки)
Source: "target\release\ExplorerTagsUI.exe"; DestDir: "{app}"; Flags: ignoreversion

; 3. Фоновый трей-агент (настройки, QuickLook, мониторинг)
Source: "target\release\explorer-tags-tray.exe"; DestDir: "{app}"; Flags: ignoreversion

; 4. Схема свойств Windows
Source: "res\PropertySchema.propdesc"; DestDir: "{app}"; Flags: ignoreversion

; 5. Папка с цветными иконками папок
Source: "Icons\*"; DestDir: "{app}\Icons"; Flags: ignoreversion recursesubdirs skipifsourcedoesntexist

[Icons]
Name: "{userstartup}\ExplorerTags Tray"; Filename: "{app}\explorer-tags-tray.exe"
Name: "{group}\Параметры ExplorerTags"; Filename: "{app}\explorer-tags-tray.exe"
Name: "{group}\ExplorerTags (Диалог)"; Filename: "{app}\ExplorerTagsUI.exe"

[Registry]
; --- 1. Регистрация КОНТЕКСТНОГО МЕНЮ (IExplorerCommand) ---
; Очистка устаревшего ключа v1.0 (устраняет дубликаты пункта контекстного меню)
Root: HKCU; Subkey: "Software\Classes\Directory\shell\ExplorerTagsMenu"; Flags: deletekey
Root: HKLM; Subkey: "Software\Classes\Directory\shell\ExplorerTagsMenu"; Flags: deletekey
Root: HKCU; Subkey: "Software\Classes\CLSID\{{A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}"; ValueType: string; ValueData: "ExplorerTags Context Menu"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\CLSID\{{A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}\InProcServer32"; ValueType: string; ValueData: "{app}\ExplorerTags.dll"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\CLSID\{{A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}\InProcServer32"; ValueName: "ThreadingModel"; ValueType: string; ValueData: "Apartment"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\shell\ExplorerTags"; ValueName: "ExplorerCommandHandler"; ValueType: string; ValueData: "{{A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\shell\ExplorerTags"; ValueName: "Icon"; ValueType: string; ValueData: "{app}\Icons\Rust 1 Folder.ico"; Flags: uninsdeletevalue

; --- 2. Регистрация ОБРАБОТЧИКА СВОЙСТВ (Property Handler IPropertyStore) ---
Root: HKCU; Subkey: "Software\Classes\CLSID\{{A1B2C3D4-2222-4444-8888-E5F6A7B8C9D1}"; ValueType: string; ValueData: "ExplorerTags Property Handler"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\CLSID\{{A1B2C3D4-2222-4444-8888-E5F6A7B8C9D1}\InProcServer32"; ValueType: string; ValueData: "{app}\ExplorerTags.dll"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\CLSID\{{A1B2C3D4-2222-4444-8888-E5F6A7B8C9D1}\InProcServer32"; ValueName: "ThreadingModel"; ValueType: string; ValueData: "Apartment"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\PropertyHandler"; ValueType: string; ValueData: "{{A1B2C3D4-2222-4444-8888-E5F6A7B8C9D1}"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Folder\PropertyHandler"; ValueType: string; ValueData: "{{A1B2C3D4-2222-4444-8888-E5F6A7B8C9D1}"; Flags: uninsdeletekey

; --- 3. Подключение колонки "Тег проекта" к списку колонок папок в Проводнике ---
Root: HKCU; Subkey: "Software\Classes\Directory"; ValueName: "FullDetails"; ValueType: string; ValueData: "prop:System.ItemNameDisplay;ExplorerTags.ProjectTag;System.ItemDateModified;System.ItemType;System.Size"; Flags: uninsdeletevalue

; --- 4. Автозапуск трей-агента при входе в Windows ---
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "ExplorerTagsTray"; ValueType: string; ValueData: """{app}\explorer-tags-tray.exe"""; Flags: uninsdeletevalue

[Run]
; Тихая регистрация схемы свойств через Rust DLL (без всплывающих окон)
Filename: "regsvr32.exe"; Parameters: "/s ""{app}\ExplorerTags.dll"""; Flags: runhidden
; Запуск трей-агента после установки
Filename: "{app}\explorer-tags-tray.exe"; Description: "Запустить ассистент ExplorerTags в панели задач"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Завершение трей-агента перед удалением
Filename: "taskkill.exe"; Parameters: "/f /im explorer-tags-tray.exe"; Flags: runhidden; RunOnceId: "KillTray"
; Тихая отмена регистрации схемы свойств
Filename: "regsvr32.exe"; Parameters: "/u /s ""{app}\ExplorerTags.dll"""; Flags: runhidden; RunOnceId: "UnregSchema"

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
begin
  if CurStep = ssInstall then
  begin
    Exec('taskkill.exe', '/f /im explorer-tags-tray.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec('taskkill.exe', '/f /im ExplorerTagsUI.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
  if CurStep = ssPostInstall then
  begin
    Exec('taskkill.exe', '/f /im dllhost.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec('taskkill.exe', '/f /im explorer.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec('explorer.exe', '', '', SW_SHOWNORMAL, ewNoWait, ResultCode);
  end;
end;

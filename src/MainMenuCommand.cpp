#include "MainMenuCommand.h"
#include "Global.h"
#include <Shlwapi.h>
#include <shlobj.h>
#include <vector>

MainMenuCommand::MainMenuCommand() : m_refCount(1) { DllAddRef(); }
MainMenuCommand::~MainMenuCommand() { DllRelease(); }

HRESULT MainMenuCommand::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = nullptr;
    if (riid == IID_IUnknown || riid == IID_IExplorerCommand) {
        *ppv = static_cast<IExplorerCommand*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}
ULONG MainMenuCommand::AddRef() { return ++m_refCount; }
ULONG MainMenuCommand::Release() { ULONG c = --m_refCount; if (c == 0) delete this; return c; }

HRESULT MainMenuCommand::GetTitle(IShellItemArray*, LPWSTR* ppszName) { return SHStrDupW(L"🎨 Настроить папку...", ppszName); }
HRESULT MainMenuCommand::GetIcon(IShellItemArray*, LPWSTR* ppszIcon) { return SHStrDupW(L"shell32.dll,274", ppszIcon); }
HRESULT MainMenuCommand::GetToolTip(IShellItemArray*, LPWSTR* ppsz) { return SHStrDupW(L"Изменить цвет и тег", ppsz); }
HRESULT MainMenuCommand::GetCanonicalName(GUID* pguid) { *pguid = GUID_NULL; return S_OK; }
HRESULT MainMenuCommand::GetState(IShellItemArray*, BOOL, EXPCMDSTATE* pCmdState) { *pCmdState = ECS_ENABLED; return S_OK; }
HRESULT MainMenuCommand::GetFlags(EXPCMDFLAGS* pFlags) { *pFlags = ECF_DEFAULT; return S_OK; }
HRESULT MainMenuCommand::EnumSubCommands(IEnumExplorerCommand** ppEnum) { *ppEnum = nullptr; return E_NOTIMPL; }

// --- ДЕЙСТВИЕ: ПРОСТО ЗАПУСКАЕМ НАШ UI.EXE ---
HRESULT MainMenuCommand::Invoke(IShellItemArray* psiItemArray, IBindCtx*) {
    if (!psiItemArray) return E_INVALIDARG;
    
    DWORD count = 0;
    psiItemArray->GetCount(&count);

    std::wstring cmdArgs = L"";
    
    // Собираем пути выделенных папок в аргументы командной строки
    for (DWORD i = 0; i < count; ++i) {
        IShellItem* pItem = nullptr;
        if (SUCCEEDED(psiItemArray->GetItemAt(i, &pItem))) {
            LPWSTR pszPath = nullptr;
            if (SUCCEEDED(pItem->GetDisplayName(SIGDN_FILESYSPATH, &pszPath))) {
                cmdArgs += L"\"" + std::wstring(pszPath) + L"\" ";
                CoTaskMemFree(pszPath);
            }
            pItem->Release();
        }
    }

    if (!cmdArgs.empty()) {
        wchar_t appDataPath[MAX_PATH];
        SHGetFolderPathW(NULL, CSIDL_APPDATA, NULL, 0, appDataPath);
        
        // Путь к нашей будущей программе интерфейса
        std::wstring exePath = std::wstring(appDataPath) + L"\\ExplorerTags\\ExplorerTagsUI.exe";

        std::wstring commandLine = L"\"" + exePath + L"\" " + cmdArgs;

        STARTUPINFOW si = { sizeof(STARTUPINFOW) };
        PROCESS_INFORMATION pi = { 0 };
        
        // Запускаем процесс АСИНХРОННО и НЕ ЖДЕМ ЕГО. Проводник свободен!
        if (CreateProcessW(NULL, const_cast<LPWSTR>(commandLine.c_str()), NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) {
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
        }
    }
    return S_OK;
}
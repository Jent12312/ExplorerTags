#include "Colorizer.h"
#include <shlobj.h>
#include <shlwapi.h>

namespace Colorizer {

static void EnsureUnicodeIniFile(const std::wstring& iniPath) {
    SetFileAttributesW(iniPath.c_str(), FILE_ATTRIBUTE_NORMAL);
    bool isUnicode = false;
    HANDLE hFileRead = CreateFileW(iniPath.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (hFileRead != INVALID_HANDLE_VALUE) {
        unsigned char bom[2] = { 0 };
        DWORD bytesRead = 0;
        ReadFile(hFileRead, bom, 2, &bytesRead, nullptr);
        CloseHandle(hFileRead);
        if (bytesRead == 2 && bom[0] == 0xFF && bom[1] == 0xFE) isUnicode = true;
    }
    if (!isUnicode) {
        HANDLE hFileWrite = CreateFileW(iniPath.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
        if (hFileWrite != INVALID_HANDLE_VALUE) {
            unsigned char bomAndNewline[] = { 0xFF, 0xFE, 0x0D, 0x00, 0x0A, 0x00 };
            DWORD bytesWritten;
            WriteFile(hFileWrite, bomAndNewline, sizeof(bomAndNewline), &bytesWritten, nullptr);
            CloseHandle(hFileWrite);
        }
    }
}

// Общая функция для обновления интерфейса
static void ApplyAttributesAndNotify(const std::wstring& folderPath, const std::wstring& iniPath) {
    SetFileAttributesW(iniPath.c_str(), FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM);
    DWORD attr = GetFileAttributesW(folderPath.c_str());
    if (attr != INVALID_FILE_ATTRIBUTES) {
        SetFileAttributesW(folderPath.c_str(), attr | FILE_ATTRIBUTE_READONLY);
    }
    SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, iniPath.c_str(), nullptr);
    SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, folderPath.c_str(), nullptr);
    
    wchar_t parentPath[MAX_PATH];
    wcscpy_s(parentPath, folderPath.c_str());
    PathRemoveFileSpecW(parentPath);
    SHChangeNotify(SHCNE_UPDATEDIR, SHCNF_PATHW, parentPath, nullptr);
}

bool SetFolderColor(const std::wstring& folderPath, const std::wstring& iconPath, int iconIndex) {
    if (folderPath.empty()) return false;
    std::wstring iniPath = folderPath + L"\\desktop.ini";
    
    EnsureUnicodeIniFile(iniPath); // Гарантируем юникод до API

    SHFOLDERCUSTOMSETTINGS fcs = {0};
    fcs.dwSize = sizeof(SHFOLDERCUSTOMSETTINGS);
    fcs.dwMask = FCSM_ICONFILE;
    fcs.pszIconFile = const_cast<LPWSTR>(iconPath.c_str());
    fcs.iIconIndex = iconIndex;
    
    SHGetSetFolderCustomSettings(&fcs, folderPath.c_str(), FCS_FORCEWRITE);
    ApplyAttributesAndNotify(folderPath, iniPath);
    return true;
}

bool SetFolderTag(const std::wstring& folderPath, const std::wstring& tagText) {
    if (folderPath.empty()) return false;
    std::wstring iniPath = folderPath + L"\\desktop.ini";
    
    EnsureUnicodeIniFile(iniPath);

    if (tagText.empty()) {
        WritePrivateProfileStringW(L".ShellClassInfo", L"InfoTip", nullptr, iniPath.c_str());
        WritePrivateProfileStringW(L"{F29F85E0-4FF9-1068-AB91-08002B27B3D9}", L"Prop5", nullptr, iniPath.c_str());
        WritePrivateProfileStringW(L"ExplorerTags", L"ProjectTag", nullptr, iniPath.c_str());
    } else {
        WritePrivateProfileStringW(L".ShellClassInfo", L"InfoTip", tagText.c_str(), iniPath.c_str());
        std::wstring propValue = L"31," + tagText;
        WritePrivateProfileStringW(L"{F29F85E0-4FF9-1068-AB91-08002B27B3D9}", L"Prop5", propValue.c_str(), iniPath.c_str());
        WritePrivateProfileStringW(L"ExplorerTags", L"ProjectTag", tagText.c_str(), iniPath.c_str());
    }
    WritePrivateProfileStringW(nullptr, nullptr, nullptr, iniPath.c_str());
    ApplyAttributesAndNotify(folderPath, iniPath);
    return true;
}

bool ResetFolderIcon(const std::wstring& folderPath) {
    if (folderPath.empty()) return false;
    SHFOLDERCUSTOMSETTINGS fcs = {0};
    fcs.dwSize = sizeof(SHFOLDERCUSTOMSETTINGS);
    fcs.dwMask = FCSM_ICONFILE | FCSM_INFOTIP;
    fcs.pszIconFile = const_cast<LPWSTR>(L"");
    fcs.iIconIndex = 0;
    fcs.pszInfoTip = const_cast<LPWSTR>(L"");
    SHGetSetFolderCustomSettings(&fcs, folderPath.c_str(), FCS_FORCEWRITE);

    std::wstring iniPath = folderPath + L"\\desktop.ini";
    WritePrivateProfileStringW(L"{F29F85E0-4FF9-1068-AB91-08002B27B3D9}", nullptr, nullptr, iniPath.c_str());
    WritePrivateProfileStringW(L"ExplorerTags", nullptr, nullptr, iniPath.c_str());
    WritePrivateProfileStringW(nullptr, nullptr, nullptr, iniPath.c_str());

    DWORD attr = GetFileAttributesW(folderPath.c_str());
    if (attr != INVALID_FILE_ATTRIBUTES) {
        SetFileAttributesW(folderPath.c_str(), attr & ~FILE_ATTRIBUTE_READONLY);
    }

    SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, folderPath.c_str(), nullptr);
    wchar_t parentPath[MAX_PATH];
    wcscpy_s(parentPath, folderPath.c_str());
    PathRemoveFileSpecW(parentPath);
    SHChangeNotify(SHCNE_UPDATEDIR, SHCNF_PATHW, parentPath, nullptr);
    return true;
}
} // namespace Colorizer
#pragma once
#include <string>
#include <windows.h>

namespace Colorizer {
    bool SetFolderColor(const std::wstring& folderPath, const std::wstring& iconPath, int iconIndex);
    bool SetFolderTag(const std::wstring& folderPath, const std::wstring& tagText);
    bool ResetFolderIcon(const std::wstring& folderPath);
}
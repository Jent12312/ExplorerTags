#pragma once
#include <string>
#include <vector>
#include <mutex>
extern "C" {
    #include "sqlite3.h"
}

struct TagInfo {
    int id;
    std::wstring name;
    std::wstring colorIcon;
};

class DatabaseManager {
public:
    static DatabaseManager& Instance();

    bool Init();
    
    // Сохранение тега для папки
    bool SetFolderTag(const std::wstring& folderPath, const std::wstring& tagName);
    
    // Получение тега папки
    std::wstring GetFolderTag(const std::wstring& folderPath);

    // Список всех пользовательских тегов
    std::vector<std::wstring> GetAllTags();

    // Добавление нового тега в глобальный список
    bool AddCustomTag(const std::wstring& tagName);
    
    // НОВОЕ: Удаление тега
    bool DeleteCustomTag(const std::wstring& tagName);

private:
    DatabaseManager();
    ~DatabaseManager();

    std::wstring GetDatabasePath();

    void* m_db; // sqlite3* pointer
    std::recursive_mutex m_mutex;
};
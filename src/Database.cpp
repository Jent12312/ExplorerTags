#include "Database.h"
#include "sqlite3.h"
#include <windows.h>
#include <shlobj.h>
#include <algorithm>

// Функция для нормализации пути (нижний регистр + удаление лишних слэшей)
static std::wstring NormalizePath(std::wstring path) {
    std::transform(path.begin(), path.end(), path.begin(), ::towlower);
    if (path.length() > 3 && path.back() == L'\\') {
        path.pop_back();
    }
    return path;
}

static std::string WideToUtf8(const std::wstring& wstr) {
    if (wstr.empty()) return std::string();
    int size = WideCharToMultiByte(CP_UTF8, 0, &wstr[0], (int)wstr.size(), NULL, 0, NULL, NULL);
    std::string strTo(size, 0);
    WideCharToMultiByte(CP_UTF8, 0, &wstr[0], (int)wstr.size(), &strTo[0], size, NULL, NULL);
    return strTo;
}

static std::wstring Utf8ToWide(const std::string& str) {
    if (str.empty()) return std::wstring();
    int size = MultiByteToWideChar(CP_UTF8, 0, &str[0], (int)str.size(), NULL, 0);
    std::wstring wstrTo(size, 0);
    MultiByteToWideChar(CP_UTF8, 0, &str[0], (int)str.size(), &wstrTo[0], size);
    return wstrTo;
}

DatabaseManager& DatabaseManager::Instance() {
    static DatabaseManager instance;
    return instance;
}

DatabaseManager::DatabaseManager() : m_db(nullptr) {}

DatabaseManager::~DatabaseManager() {
    if (m_db) {
        sqlite3_close((sqlite3*)m_db);
    }
}

std::wstring DatabaseManager::GetDatabasePath() {
    wchar_t appDataPath[MAX_PATH];
    if (SUCCEEDED(SHGetFolderPathW(NULL, CSIDL_APPDATA, NULL, 0, appDataPath))) {
        std::wstring dir = std::wstring(appDataPath) + L"\\ExplorerTags";
        CreateDirectoryW(dir.c_str(), NULL);
        return dir + L"\\database.db";
    }
    return L"database.db";
}

bool DatabaseManager::Init() {
    std::lock_guard<std::recursive_mutex> lock(m_mutex);
    if (m_db) return true;

    std::wstring dbPath = GetDatabasePath();
    std::string dbPathUtf8 = WideToUtf8(dbPath);

    sqlite3* db = nullptr;
    if (sqlite3_open(dbPathUtf8.c_str(), &db) != SQLITE_OK) {
        return false;
    }
    m_db = db;

    // Включаем режим WAL, чтобы explorer.exe и dllhost.exe не блокировали друг друга!
    sqlite3_exec(db, "PRAGMA journal_mode=WAL;", nullptr, nullptr, nullptr);
    
    // Устанавливаем таймаут ожидания (1 секунда вместо бесконечного зависания)
    sqlite3_exec(db, "PRAGMA busy_timeout=1000;", nullptr, nullptr, nullptr);

    const char* sqlCreate = 
        "CREATE TABLE IF NOT EXISTS custom_tags (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE NOT NULL);"
        "CREATE TABLE IF NOT EXISTS folder_tags (path TEXT PRIMARY KEY, tag TEXT NOT NULL);";
    sqlite3_exec(db, sqlCreate, nullptr, nullptr, nullptr);

    // Добавляем стартовые теги только если таблица пустая
    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2(db, "SELECT COUNT(*) FROM custom_tags", -1, &stmt, nullptr) == SQLITE_OK) {
        if (sqlite3_step(stmt) == SQLITE_ROW && sqlite3_column_int(stmt, 0) == 0) {
            sqlite3_exec(db, "INSERT INTO custom_tags (name) VALUES ('🔥 Срочно'), ('⭐ Важно'), ('📦 Архив');", nullptr, nullptr, nullptr);
        }
        sqlite3_finalize(stmt);
    }

    char* err = nullptr;
    sqlite3_exec(db, sqlCreate, nullptr, nullptr, &err);

    return true;
}

bool DatabaseManager::SetFolderTag(const std::wstring& folderPath, const std::wstring& tagName) {
    Init();
    std::lock_guard<std::recursive_mutex> lock(m_mutex);

    std::string pathUtf8 = WideToUtf8(NormalizePath(folderPath));
    std::string tagUtf8 = WideToUtf8(tagName);

    const char* sql = "INSERT INTO folder_tags (path, tag) VALUES (?, ?) "
                      "ON CONFLICT(path) DO UPDATE SET tag = excluded.tag;";

    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2((sqlite3*)m_db, sql, -1, &stmt, nullptr) == SQLITE_OK) {
        sqlite3_bind_text(stmt, 1, pathUtf8.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(stmt, 2, tagUtf8.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        return true;
    }
    return false;
}

std::wstring DatabaseManager::GetFolderTag(const std::wstring& folderPath) {
    Init();
    std::lock_guard<std::recursive_mutex> lock(m_mutex);

    std::string pathUtf8 = WideToUtf8(NormalizePath(folderPath));
    const char* sql = "SELECT tag FROM folder_tags WHERE path = ?;";
    
    sqlite3_stmt* stmt = nullptr;
    std::wstring result = L"";

    if (sqlite3_prepare_v2((sqlite3*)m_db, sql, -1, &stmt, nullptr) == SQLITE_OK) {
        sqlite3_bind_text(stmt, 1, pathUtf8.c_str(), -1, SQLITE_TRANSIENT);
        if (sqlite3_step(stmt) == SQLITE_ROW) {
            const unsigned char* text = sqlite3_column_text(stmt, 0);
            if (text) {
                result = Utf8ToWide((const char*)text);
            }
        }
        sqlite3_finalize(stmt);
    }
    return result;
}

bool DatabaseManager::AddCustomTag(const std::wstring& tagName) {
    Init();
    std::lock_guard<std::recursive_mutex> lock(m_mutex);

    std::string tagUtf8 = WideToUtf8(tagName);

    const char* sql = "INSERT OR IGNORE INTO custom_tags (name) VALUES (?);";
    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2((sqlite3*)m_db, sql, -1, &stmt, nullptr) == SQLITE_OK) {
        sqlite3_bind_text(stmt, 1, tagUtf8.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        return true;
    }
    return false;
}

std::vector<std::wstring> DatabaseManager::GetAllTags() {
    Init();
    std::lock_guard<std::recursive_mutex> lock(m_mutex);

    std::vector<std::wstring> tags;
    const char* sql = "SELECT name FROM custom_tags ORDER BY id ASC;";
    sqlite3_stmt* stmt = nullptr;

    if (sqlite3_prepare_v2((sqlite3*)m_db, sql, -1, &stmt, nullptr) == SQLITE_OK) {
        while (sqlite3_step(stmt) == SQLITE_ROW) {
            const unsigned char* text = sqlite3_column_text(stmt, 0);
            if (text) {
                tags.push_back(Utf8ToWide((const char*)text));
            }
        }
        sqlite3_finalize(stmt);
    }
    return tags;
}

bool DatabaseManager::DeleteCustomTag(const std::wstring& tagName) {
    Init();
    std::lock_guard<std::recursive_mutex> lock(m_mutex);

    std::string tagUtf8 = WideToUtf8(tagName);
    const char* sql = "DELETE FROM custom_tags WHERE name = ?;";
    
    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2((sqlite3*)m_db, sql, -1, &stmt, nullptr) == SQLITE_OK) {
        sqlite3_bind_text(stmt, 1, tagUtf8.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        return true;
    }
    return false;
}
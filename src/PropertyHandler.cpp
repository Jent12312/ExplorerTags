#include "PropertyHandler.h"
#include "Global.h"
#include <propvarutil.h>

// Определяем ваш кастомный ключ из PropertySchema.propdesc
// formatID="{B2D3E4F5-6666-7777-9999-A1B2C3D4E5F6}" propID="100"
const PROPERTYKEY PKEY_ExplorerTags_ProjectTag = { 
    { 0xB2D3E4F5, 0x6666, 0x7777, { 0x99, 0x99, 0xA1, 0xB2, 0xC3, 0xD4, 0xE5, 0xF6 } }, 100 
};

PropertyHandler::PropertyHandler() : m_refCount(1) { DllAddRef(); }
PropertyHandler::~PropertyHandler() { DllRelease(); }

HRESULT PropertyHandler::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = nullptr;
    if (riid == IID_IUnknown || riid == IID_IPropertyStore) {
        *ppv = static_cast<IPropertyStore*>(this);
        AddRef();
        return S_OK;
    } else if (riid == IID_IInitializeWithItem) {
        *ppv = static_cast<IInitializeWithItem*>(this);
        AddRef();
        return S_OK;
    } else if (riid == IID_IInitializeWithFile) {
        *ppv = static_cast<IInitializeWithFile*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}

ULONG PropertyHandler::AddRef() { return ++m_refCount; }
ULONG PropertyHandler::Release() { ULONG c = --m_refCount; if (c == 0) delete this; return c; }

HRESULT PropertyHandler::Initialize(IShellItem* psi, DWORD) {
    if (!psi) return E_INVALIDARG;
    LPWSTR pszPath = nullptr;
    HRESULT hr = psi->GetDisplayName(SIGDN_FILESYSPATH, &pszPath);
    if (SUCCEEDED(hr)) {
        m_folderPath = pszPath;
        CoTaskMemFree(pszPath);
    }
    return hr;
}

HRESULT PropertyHandler::Initialize(LPCWSTR pszFilePath, DWORD) {
    if (!pszFilePath) return E_INVALIDARG;
    m_folderPath = pszFilePath;
    return S_OK;
}

// Указываем, что мы поддерживаем ДВА свойства: Стандартные теги Windows и вашу колонку
HRESULT PropertyHandler::GetCount(DWORD* cProps) {
    if (!cProps) return E_POINTER;
    *cProps = 2; 
    return S_OK;
}

HRESULT PropertyHandler::GetAt(DWORD iProp, PROPERTYKEY* pkey) {
    if (!pkey) return E_POINTER;
    if (iProp == 0) {
        *pkey = PKEY_Keywords; // Стандартная колонка "Теги"
        return S_OK;
    } else if (iProp == 1) {
        *pkey = PKEY_ExplorerTags_ProjectTag; // Ваша кастомная колонка "Тег проекта"
        return S_OK;
    }
    return E_INVALIDARG;
}

HRESULT PropertyHandler::GetValue(REFPROPERTYKEY key, PROPVARIANT* ppropvar) {
    if (!ppropvar) return E_POINTER;
    PropVariantInit(ppropvar);

    if (IsEqualPropertyKey(key, PKEY_Keywords) || IsEqualPropertyKey(key, PKEY_ExplorerTags_ProjectTag)) {
        
        // БОЛЬШЕ НИКАКОЙ БАЗЫ ДАННЫХ! Читаем прямо из desktop.ini со скоростью света.
        std::wstring iniPath = m_folderPath + L"\\desktop.ini";
        
        wchar_t tagBuffer[256] = {0};
        GetPrivateProfileStringW(L"ExplorerTags", L"ProjectTag", L"", tagBuffer, 256, iniPath.c_str());
        
        // Если тег найден, отдаем его Проводнику
        if (tagBuffer[0] != L'\0') {
            PCWSTR tags[] = { tagBuffer };
            return InitPropVariantFromStringVector(tags, 1, ppropvar);
        }
    }

    return S_OK; 
}

HRESULT PropertyHandler::SetValue(REFPROPERTYKEY, REFPROPVARIANT) { return STG_E_ACCESSDENIED; }
HRESULT PropertyHandler::Commit() { return S_OK; }
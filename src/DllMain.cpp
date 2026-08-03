#include "Global.h"
#include <windows.h>
#include <Shlwapi.h> // Потребуется позже
#include <wrl/client.h> // Modern COM (Microsoft WRL)
#include "Guids.h"           
#include "ClassFactory.h"
#include "MainMenuCommand.h"
#include <propsys.h>
#include <string> // Убедитесь, что это есть наверху файла
#include "PropertyHandler.h"

// Точка входа DLL
BOOL APIENTRY DllMain(HMODULE hModule, DWORD ul_reason_for_call, LPVOID /*lpReserved*/) {
    switch (ul_reason_for_call) {
    case DLL_PROCESS_ATTACH:
        g_hInst = hModule;
        // Отключаем уведомления о потоках для оптимизации производительности
        DisableThreadLibraryCalls(hModule);
        break;
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
    case DLL_PROCESS_DETACH:
        break;
    }
    return TRUE;
}

HRESULT CreateContextMenu(REFIID riid, void** ppv) {
    // Создаем наш объект меню
    MainMenuCommand* pCommand = new (std::nothrow) MainMenuCommand();
    if (!pCommand) return E_OUTOFMEMORY;

    // Запрашиваем у него интерфейс и возвращаем Проводнику
    HRESULT hr = pCommand->QueryInterface(riid, ppv);
    pCommand->Release();
    return hr;
}

HRESULT CreatePropertyHandler(REFIID riid, void** ppv) {
    PropertyHandler* pHandler = new (std::nothrow) PropertyHandler();
    if (!pHandler) return E_OUTOFMEMORY;

    HRESULT hr = pHandler->QueryInterface(riid, ppv);
    pHandler->Release();
    return hr;
}

// Проводник спрашивает: "Можно ли выгрузить твою DLL из памяти?"
STDAPI DllCanUnloadNow() {
    // Если счетчик равен 0, разрешаем (S_OK), иначе запрещаем (S_FALSE)
    return (g_DllRefCount == 0) ? S_OK : S_FALSE;
}

// Проводник просит создать объект (фабрику классов) по GUID
STDAPI DllGetClassObject(REFCLSID rclsid, REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = nullptr;

    CreateInstanceFn createFn = nullptr;

    // Проверяем, какой именно компонент хочет создать Проводник
    if (rclsid == CLSID_ExplorerTagsMenu) {
        createFn = CreateContextMenu;
    } else if (rclsid == CLSID_ExplorerTagsPropHandler) {
        createFn = CreatePropertyHandler;
    } else {
        // Если Проводник просит GUID, о котором мы не знаем
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    // Создаем нашу фабрику (std::nothrow гарантирует, что при нехватке памяти 
    // не вылетит C++ exception, который уронит Проводник)
    ClassFactory* pFactory = new (std::nothrow) ClassFactory(createFn);
    if (!pFactory) return E_OUTOFMEMORY;

    // Просим фабрику отдать интерфейс (например, IClassFactory), который просит Проводник
    HRESULT hr = pFactory->QueryInterface(riid, ppv);
    
    // Вызываем Release(), так как QueryInterface уже сделал AddRef() внутри себя,
    // иначе фабрика никогда не удалится из памяти (утечка памяти)
    pFactory->Release();

    return hr;
}

// Регистрация DLL в реестре Windows


STDAPI DllRegisterServer() {
    wchar_t dllPath[MAX_PATH];
    GetModuleFileNameW(g_hInst, dllPath, MAX_PATH);

    std::wstring pathStr(dllPath);
    size_t lastSlash = pathStr.find_last_of(L"\\/");
    
    if (lastSlash != std::wstring::npos) {
        std::wstring dirPath = pathStr.substr(0, lastSlash);
        std::wstring schemaPath = dirPath + L"\\PropertySchema.propdesc";

        // 1. Проверяем, видит ли код файл схемы физически
        if (GetFileAttributesW(schemaPath.c_str()) == INVALID_FILE_ATTRIBUTES) {
            std::wstring msg = L"ОШИБКА: Файл PropertySchema.propdesc НЕ НАЙДЕН по пути:\n" + schemaPath;
            MessageBoxW(nullptr, msg.c_str(), L"Диагностика", MB_OK | MB_ICONERROR);
            return E_FAIL;
        }

        // 2. Пытаемся зарегистрировать схему
        HRESULT hr = PSRegisterPropertySchema(schemaPath.c_str());
        
        if (FAILED(hr)) {
            // Если ошибка, выводим её HEX-код (например, 0x80070005 - это отказ в доступе)
            wchar_t hexCode[32];
            swprintf_s(hexCode, L"0x%08X", hr);
            std::wstring msg = L"ОШИБКА: Windows отказалась регистрировать схему!\nКод ошибки: " + std::wstring(hexCode);
            MessageBoxW(nullptr, msg.c_str(), L"Диагностика", MB_OK | MB_ICONERROR);
            return hr;
        } else {
            MessageBoxW(nullptr, L"УСПЕХ! Схема свойств официально зарегистрирована в Windows!", L"Диагностика", MB_OK | MB_ICONINFORMATION);
        }
    }

    return S_OK;
}
// Удаление DLL из реестра Windows
STDAPI DllUnregisterServer() {
    // TODO: Код удаления записей из реестра
    return S_OK;
}


#include "ClassFactory.h"
#include "Global.h"

// При создании фабрики увеличиваем глобальный счетчик DLL
ClassFactory::ClassFactory(CreateInstanceFn createFn) : m_refCount(1), m_createFn(createFn) {
    DllAddRef();
}

// При уничтожении фабрики уменьшаем глобальный счетчик DLL
ClassFactory::~ClassFactory() {
    DllRelease();
}

// Проводник спрашивает фабрику, поддерживает ли она нужный ему интерфейс
HRESULT ClassFactory::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = nullptr;

    if (riid == IID_IUnknown || riid == IID_IClassFactory) {
        *ppv = static_cast<IClassFactory*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}

ULONG ClassFactory::AddRef() {
    return ++m_refCount;
}

ULONG ClassFactory::Release() {
    ULONG count = --m_refCount;
    if (count == 0) {
        delete this; // Безопасное самоуничтожение объекта
    }
    return count;
}

// Проводник просит фабрику создать конечный объект (например, меню)
HRESULT ClassFactory::CreateInstance(IUnknown* pUnkOuter, REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = nullptr;

    // Мы не поддерживаем агрегацию (особенность COM), поэтому возвращаем ошибку, если pUnkOuter не null
    if (pUnkOuter) return CLASS_E_NOAGGREGATION;

    // Вызываем функцию создания, которую передали в конструктор
    return m_createFn(riid, ppv);
}

// Блокировка выгрузки DLL (редко используется, но реализовать нужно)
HRESULT ClassFactory::LockServer(BOOL fLock) {
    if (fLock) {
        DllAddRef();
    } else {
        DllRelease();
    }
    return S_OK;
}
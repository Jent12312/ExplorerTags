#pragma once
#include <unknwn.h>
#include <atomic>

// Тип указателя на функцию, которая будет создавать наши объекты
using CreateInstanceFn = HRESULT(*)(REFIID, void**);

class ClassFactory : public IClassFactory {
public:
    // Конструктор принимает функцию, которая знает, как создать конкретный объект
    explicit ClassFactory(CreateInstanceFn createFn);
    ~ClassFactory();

    // Методы базового интерфейса IUnknown
    IFACEMETHODIMP QueryInterface(REFIID riid, void** ppv) override;
    IFACEMETHODIMP_(ULONG) AddRef() override;
    IFACEMETHODIMP_(ULONG) Release() override;

    // Методы интерфейса IClassFactory
    IFACEMETHODIMP CreateInstance(IUnknown* pUnkOuter, REFIID riid, void** ppv) override;
    IFACEMETHODIMP LockServer(BOOL fLock) override;

private:
    std::atomic<long> m_refCount;   // Счетчик ссылок конкретно этой фабрики
    CreateInstanceFn m_createFn;    // Функция создания целевого объекта
};
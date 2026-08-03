#pragma once
#include <propsys.h>
#include <propkey.h>
#include <shobjidl.h>
#include <atomic>
#include <string>

class PropertyHandler : public IPropertyStore, public IInitializeWithItem, public IInitializeWithFile {
public:
    PropertyHandler();
    ~PropertyHandler();

    // IUnknown
    IFACEMETHODIMP QueryInterface(REFIID riid, void** ppv) override;
    IFACEMETHODIMP_(ULONG) AddRef() override;
    IFACEMETHODIMP_(ULONG) Release() override;

    // IInitializeWithItem (Инициализация через IShellItem)
    IFACEMETHODIMP Initialize(IShellItem* psi, DWORD mode) override;

    // IInitializeWithFile (Инициализация напрямую через путь к файлу/папке)
    IFACEMETHODIMP Initialize(LPCWSTR pszFilePath, DWORD grfMode) override;

    // IPropertyStore
    IFACEMETHODIMP GetCount(DWORD* cProps) override;
    IFACEMETHODIMP GetAt(DWORD iProp, PROPERTYKEY* pkey) override;
    IFACEMETHODIMP GetValue(REFPROPERTYKEY key, PROPVARIANT* ppropvar) override;
    IFACEMETHODIMP SetValue(REFPROPERTYKEY key, REFPROPVARIANT propvar) override;
    IFACEMETHODIMP Commit() override;

private:
    std::atomic<long> m_refCount;
    std::wstring m_folderPath;
};
#pragma once
#include <shobjidl_core.h>
#include <vector>
#include <atomic>

class CommandEnum : public IEnumExplorerCommand {
public:
    CommandEnum(const std::vector<IExplorerCommand*>& commands);
    ~CommandEnum();

    IFACEMETHODIMP QueryInterface(REFIID riid, void** ppv) override;
    IFACEMETHODIMP_(ULONG) AddRef() override;
    IFACEMETHODIMP_(ULONG) Release() override;

    IFACEMETHODIMP Next(ULONG celt, IExplorerCommand** pUICommand, ULONG* pceltFetched) override;
    IFACEMETHODIMP Skip(ULONG celt) override;
    IFACEMETHODIMP Reset() override;
    IFACEMETHODIMP Clone(IEnumExplorerCommand** ppenum) override;

private:
    std::atomic<long> m_refCount;
    std::vector<IExplorerCommand*> m_commands;
    ULONG m_current;
};
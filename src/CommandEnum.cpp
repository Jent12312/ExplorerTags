#include "CommandEnum.h"
#include "Global.h"

CommandEnum::CommandEnum(const std::vector<IExplorerCommand*>& commands) 
    : m_refCount(1), m_commands(commands), m_current(0) {
    DllAddRef();
    // Увеличиваем счетчик ссылок для каждого пункта меню
    for (auto cmd : m_commands) {
        cmd->AddRef();
    }
}

CommandEnum::~CommandEnum() {
    for (auto cmd : m_commands) {
        cmd->Release();
    }
    DllRelease();
}

HRESULT CommandEnum::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = nullptr;
    if (riid == IID_IUnknown || riid == IID_IEnumExplorerCommand) {
        *ppv = static_cast<IEnumExplorerCommand*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}

ULONG CommandEnum::AddRef() { return ++m_refCount; }
ULONG CommandEnum::Release() { ULONG c = --m_refCount; if (c == 0) delete this; return c; }

HRESULT CommandEnum::Next(ULONG celt, IExplorerCommand** pUICommand, ULONG* pceltFetched) {
    ULONG fetched = 0;
    while (m_current < m_commands.size() && fetched < celt) {
        pUICommand[fetched] = m_commands[m_current];
        pUICommand[fetched]->AddRef();
        m_current++;
        fetched++;
    }
    if (pceltFetched) *pceltFetched = fetched;
    return (fetched == celt) ? S_OK : S_FALSE;
}

HRESULT CommandEnum::Skip(ULONG celt) {
    m_current += celt;
    if (m_current > m_commands.size()) m_current = (ULONG)m_commands.size();
    return S_OK;
}

HRESULT CommandEnum::Reset() {
    m_current = 0;
    return S_OK;
}

HRESULT CommandEnum::Clone(IEnumExplorerCommand** ppenum) {
    *ppenum = new (std::nothrow) CommandEnum(m_commands);
    return *ppenum ? S_OK : E_OUTOFMEMORY;
}
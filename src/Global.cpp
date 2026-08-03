#include "Global.h"

std::atomic<long> g_DllRefCount{0};
HINSTANCE g_hInst = nullptr;

void DllAddRef() {
    ++g_DllRefCount;
}

void DllRelease() {
    --g_DllRefCount;
}
#pragma once
#include <atomic>
#include <windows.h>

// Глобальный счетчик запущенных компонентов нашей DLL
extern std::atomic<long> g_DllRefCount;

// Хэндл нашего модуля (нужен для загрузки ресурсов, иконок)
extern HINSTANCE g_hInst;

// Удобные функции для управления временем жизни DLL
void DllAddRef();
void DllRelease();
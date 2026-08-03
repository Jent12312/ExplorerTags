#pragma once
#include <initguid.h> // Обязательно для макроса DEFINE_GUID

// GUID для нашего Контекстного меню (IExplorerCommand)
// {A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}
DEFINE_GUID(CLSID_ExplorerTagsMenu, 
    0xa1b2c3d4, 0x1111, 0x4444, 0x88, 0x88, 0xe5, 0xf6, 0xa7, 0xb8, 0xc9, 0xd0);

// GUID для нашего Обработчика свойств (Property Handler)
// {A1B2C3D4-2222-4444-8888-E5F6A7B8C9D1}
DEFINE_GUID(CLSID_ExplorerTagsPropHandler, 
    0xa1b2c3d4, 0x2222, 0x4444, 0x88, 0x88, 0xe5, 0xf6, 0xa7, 0xb8, 0xc9, 0xd1);
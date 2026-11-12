use windows::core::GUID;
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

/// GUID для контекстного меню Проводника (IExplorerCommand)
/// {A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}
pub const CLSID_EXPLORER_TAGS_MENU: GUID = GUID::from_u128(0xa1b2c3d4_1111_4444_8888_e5f6a7b8c9d0);

/// GUID для обработчика свойств Windows (Property Handler IPropertyStore)
/// {A1B2C3D4-2222-4444-8888-E5F6A7B8C9D1}
pub const CLSID_EXPLORER_TAGS_PROP_HANDLER: GUID = GUID::from_u128(0xa1b2c3d4_2222_4444_8888_e5f6a7b8c9d1);

/// formatID="{B2D3E4F5-6666-7777-9999-A1B2C3D4E5F6}"
pub const FMTID_EXPLORER_TAGS: GUID = GUID::from_u128(0xb2d3e4f5_6666_7777_9999_a1b2c3d4e5f6);

/// Свойство "Тег проекта" (PID 100)
pub const PKEY_EXPLORER_TAGS_PROJECT_TAG: PROPERTYKEY = PROPERTYKEY {
    fmtid: FMTID_EXPLORER_TAGS,
    pid: 100,
};

/// Системное свойство Windows "Keywords" (Теги): {F29F85E0-4FF9-1068-AB91-08002B27B3D9}, pid: 5
pub const FMTID_SUMMARY_INFORMATION: GUID = GUID::from_u128(0xf29f85e0_4ff9_1068_ab91_08002b27b3d9);
pub const PKEY_KEYWORDS: PROPERTYKEY = PROPERTYKEY {
    fmtid: FMTID_SUMMARY_INFORMATION,
    pid: 5,
};

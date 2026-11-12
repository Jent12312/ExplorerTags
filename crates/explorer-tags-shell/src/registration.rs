use std::path::PathBuf;
use windows::core::{PCWSTR, Result, HRESULT};
use windows::Win32::Foundation::{HMODULE, S_OK};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_ALL_ACCESS, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::Shell::PropertiesSystem::{
    PSRegisterPropertySchema, PSUnregisterPropertySchema,
};

const MENU_CLSID_STR: &str = "{A1B2C3D4-1111-4444-8888-E5F6A7B8C9D0}";
const PROP_CLSID_STR: &str = "{A1B2C3D4-2222-4444-8888-E5F6A7B8C9D1}";

pub fn register_server(hmodule: HMODULE) -> HRESULT {
    let dll_path = match get_module_path(hmodule) {
        Some(p) => p,
        None => return windows::Win32::Foundation::E_FAIL,
    };

    let dll_path_str = dll_path.to_string_lossy().to_string();

    // 1. Регистрируем схему свойств (тихо, без всплывающих окон)
    if let Some(schema_path) = find_schema_path(&dll_path) {
        let schema_wide: Vec<u16> = schema_path
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            PSRegisterPropertySchema(PCWSTR(schema_wide.as_ptr())).ok();
        }
    }

    // 2. Регистрируем COM-классы в HKCU\Software\Classes
    let root = HKEY_CURRENT_USER;
    let _ = reg_set_string(
        root,
        &format!("Software\\Classes\\CLSID\\{MENU_CLSID_STR}"),
        "",
        "ExplorerTags Context Menu",
    );
    let _ = reg_set_string(
        root,
        &format!("Software\\Classes\\CLSID\\{MENU_CLSID_STR}\\InProcServer32"),
        "",
        &dll_path_str,
    );
    let _ = reg_set_string(
        root,
        &format!("Software\\Classes\\CLSID\\{MENU_CLSID_STR}\\InProcServer32"),
        "ThreadingModel",
        "Apartment",
    );

    let _ = reg_set_string(
        root,
        &format!("Software\\Classes\\CLSID\\{PROP_CLSID_STR}"),
        "",
        "ExplorerTags Property Handler",
    );
    let _ = reg_set_string(
        root,
        &format!("Software\\Classes\\CLSID\\{PROP_CLSID_STR}\\InProcServer32"),
        "",
        &dll_path_str,
    );
    let _ = reg_set_string(
        root,
        &format!("Software\\Classes\\CLSID\\{PROP_CLSID_STR}\\InProcServer32"),
        "ThreadingModel",
        "Apartment",
    );

    // 3. Регистрируем пункт контекстного меню
    // Очищаем устаревший ключ от v1.0, чтобы не было дубликатов
    let _ = reg_delete_tree(root, "Software\\Classes\\Directory\\shell\\ExplorerTagsMenu");
    let _ = reg_delete_tree(windows::Win32::System::Registry::HKEY_LOCAL_MACHINE, "Software\\Classes\\Directory\\shell\\ExplorerTagsMenu");

    let _ = reg_set_string(
        root,
        "Software\\Classes\\Directory\\shell\\ExplorerTags",
        "ExplorerCommandHandler",
        MENU_CLSID_STR,
    );

    let icon_val = if let Some(ico) = crate::menu_command::find_menu_icon_path() {
        ico.to_string_lossy().to_string()
    } else {
        "shell32.dll,3".to_string()
    };
    let _ = reg_set_string(
        root,
        "Software\\Classes\\Directory\\shell\\ExplorerTags",
        "Icon",
        &icon_val,
    );

    // 4. Регистрируем Property Handler для каталогов
    let _ = reg_set_string(
        root,
        "Software\\Classes\\Directory\\PropertyHandler",
        "",
        PROP_CLSID_STR,
    );
    let _ = reg_set_string(
        root,
        "Software\\Classes\\Folder\\PropertyHandler",
        "",
        PROP_CLSID_STR,
    );

    S_OK
}

pub fn unregister_server(hmodule: HMODULE) -> HRESULT {
    let dll_path = get_module_path(hmodule);

    // 1. Снимаем регистрацию схемы свойств
    if let Some(dll_p) = &dll_path {
        if let Some(schema_path) = find_schema_path(dll_p) {
            let schema_wide: Vec<u16> = schema_path
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                PSUnregisterPropertySchema(PCWSTR(schema_wide.as_ptr())).ok();
            }
        }
    }

    // 2. Удаляем ключи из HKCU
    let root = HKEY_CURRENT_USER;
    let _ = reg_delete_tree(root, &format!("Software\\Classes\\CLSID\\{MENU_CLSID_STR}"));
    let _ = reg_delete_tree(root, &format!("Software\\Classes\\CLSID\\{PROP_CLSID_STR}"));
    let _ = reg_delete_tree(root, "Software\\Classes\\Directory\\shell\\ExplorerTags");
    let _ = reg_delete_tree(root, "Software\\Classes\\Directory\\shell\\ExplorerTagsMenu");
    let _ = reg_delete_tree(root, "Software\\Classes\\Directory\\PropertyHandler");
    let _ = reg_delete_tree(root, "Software\\Classes\\Folder\\PropertyHandler");

    S_OK
}

fn get_module_path(hmodule: HMODULE) -> Option<PathBuf> {
    unsafe {
        let mut buf = [0u16; 1024];
        let len = GetModuleFileNameW(hmodule, &mut buf);
        if len > 0 {
            Some(PathBuf::from(String::from_utf16_lossy(&buf[..len as usize])))
        } else {
            None
        }
    }
}

fn find_schema_path(dll_path: &std::path::Path) -> Option<PathBuf> {
    let mut current = dll_path.parent();
    while let Some(dir) = current {
        let candidate1 = dir.join("PropertySchema.propdesc");
        if candidate1.is_file() {
            return Some(candidate1);
        }
        let candidate2 = dir.join("res").join("PropertySchema.propdesc");
        if candidate2.is_file() {
            return Some(candidate2);
        }
        current = dir.parent();
    }
    None
}

fn reg_set_string(root: HKEY, subkey: &str, value_name: &str, data: &str) -> Result<()> {
    let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let val_name_w: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
    let data_w: Vec<u16> = data.encode_utf16().chain(std::iter::once(0)).collect();

    let mut hkey = HKEY::default();
    unsafe {
        RegCreateKeyExW(
            root,
            PCWSTR(subkey_w.as_ptr()),
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_ALL_ACCESS,
            None,
            &mut hkey,
            None,
        ).ok()?;

        let bytes = std::slice::from_raw_parts(
            data_w.as_ptr() as *const u8,
            data_w.len() * std::mem::size_of::<u16>(),
        );

        RegSetValueExW(
            hkey,
            if value_name.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(val_name_w.as_ptr())
            },
            0,
            REG_SZ,
            Some(bytes),
        ).ok()?;

        let _ = RegCloseKey(hkey);
    }

    Ok(())
}

fn reg_delete_tree(root: HKEY, subkey: &str) -> Result<()> {
    let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = RegDeleteTreeW(root, PCWSTR(subkey_w.as_ptr()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_helpers() {
        let test_key = "Software\\Classes\\ExplorerTagsTestKey";
        let res = reg_set_string(HKEY_CURRENT_USER, test_key, "TestVal", "Hello");
        assert!(res.is_ok());
        let del = reg_delete_tree(HKEY_CURRENT_USER, test_key);
        assert!(del.is_ok());
    }
}

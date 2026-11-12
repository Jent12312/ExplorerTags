use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "ExplorerTagsTray";

pub fn is_autostart_enabled() -> bool {
    let subkey_w: Vec<u16> = RUN_SUBKEY.encode_utf16().chain(std::iter::once(0)).collect();
    let val_name_w: Vec<u16> = VALUE_NAME.encode_utf16().chain(std::iter::once(0)).collect();

    let mut hkey = windows::Win32::System::Registry::HKEY::default();
    let open_res = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_w.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        )
    };

    if open_res.is_err() {
        return false;
    }

    let mut buffer = [0u16; 512];
    let mut size = (buffer.len() * 2) as u32;
    let query_res = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(val_name_w.as_ptr()),
            None,
            None,
            Some(buffer.as_mut_ptr() as _),
            Some(&mut size),
        )
    };

    unsafe {
        let _ = RegCloseKey(hkey);
    }

    query_res.is_ok()
}

pub fn set_autostart_enabled(enable: bool) -> Result<(), String> {
    let subkey_w: Vec<u16> = RUN_SUBKEY.encode_utf16().chain(std::iter::once(0)).collect();
    let val_name_w: Vec<u16> = VALUE_NAME.encode_utf16().chain(std::iter::once(0)).collect();

    let mut hkey = windows::Win32::System::Registry::HKEY::default();

    if enable {
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_str = format!("\"{}\"", exe_path.to_string_lossy());
        let exe_w: Vec<u16> = exe_str.encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            let res = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey_w.as_ptr()),
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            );
            if res.is_err() {
                return Err(format!("RegCreateKeyExW failed: {:?}", res));
            }

            let set_res = RegSetValueExW(
                hkey,
                PCWSTR(val_name_w.as_ptr()),
                0,
                REG_SZ,
                Some(std::slice::from_raw_parts(
                    exe_w.as_ptr() as *const u8,
                    exe_w.len() * 2,
                )),
            );
            let _ = RegCloseKey(hkey);

            if set_res.is_err() {
                return Err(format!("RegSetValueExW failed: {:?}", set_res));
            }
        }
    } else {
        unsafe {
            if RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey_w.as_ptr()),
                0,
                KEY_WRITE,
                &mut hkey,
            )
            .is_ok()
            {
                let _ = RegDeleteValueW(hkey, PCWSTR(val_name_w.as_ptr()));
                let _ = RegCloseKey(hkey);
            }
        }
    }

    Ok(())
}

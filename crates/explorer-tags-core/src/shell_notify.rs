use std::path::Path;
use windows::Win32::UI::Shell::{
    SHChangeNotify, SHCNE_UPDATEDIR, SHCNE_UPDATEITEM, SHCNF_PATHW,
};

/// Уведомляет Проводник Windows о том, что папка и её родительский каталог обновились
pub fn notify_shell_folder_updated<P: AsRef<Path>>(folder_path: P) {
    let p = folder_path.as_ref();
    let folder_w = to_wide_null(p.to_str().unwrap_or_default());
    let ini_path = p.join("desktop.ini");
    let ini_w = to_wide_null(ini_path.to_str().unwrap_or_default());

    unsafe {
        // 1. Уведомляем об обновлении desktop.ini
        SHChangeNotify(
            SHCNE_UPDATEITEM,
            SHCNF_PATHW,
            Some(ini_w.as_ptr() as _),
            None,
        );

        // 2. Уведомляем об обновлении самой папки
        SHChangeNotify(
            SHCNE_UPDATEITEM,
            SHCNF_PATHW,
            Some(folder_w.as_ptr() as _),
            None,
        );

        // 3. Уведомляем родительский каталог (чтобы обновилось отображение иконки в списке файлов)
        if let Some(parent) = p.parent() {
            let parent_w = to_wide_null(parent.to_str().unwrap_or_default());
            SHChangeNotify(
                SHCNE_UPDATEDIR,
                SHCNF_PATHW,
                Some(parent_w.as_ptr() as _),
                None,
            );
        }
    }
}

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

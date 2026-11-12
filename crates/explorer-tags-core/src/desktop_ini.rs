use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL,
    FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_SYSTEM, INVALID_FILE_ATTRIBUTES,
};
use windows::Win32::UI::Shell::{
    SHGetSetFolderCustomSettings, FCSM_ICONFILE, FCSM_INFOTIP, FCS_FORCEWRITE,
    SHFOLDERCUSTOMSETTINGS,
};

use crate::shell_notify::notify_shell_folder_updated;

const SHELL_CLASS_INFO: &str = ".ShellClassInfo";
const SUMMARY_INFO_SECTION: &str = "{F29F85E0-4FF9-1068-AB91-08002B27B3D9}";
const EXPLORER_TAGS_SECTION: &str = "ExplorerTags";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderState {
    pub folder_path: PathBuf,
    pub tag: Option<String>,
    pub icon_path: Option<String>,
}

/// Простая структура для манипуляций с секциями INI-файла
#[derive(Debug, Clone, Default)]
pub struct IniDocument {
    pub sections: BTreeMap<String, BTreeMap<String, String>>,
}

impl IniDocument {
    pub fn parse(text: &str) -> Self {
        let mut sections: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut current_section = String::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len() - 1].trim().to_string();
                sections.entry(current_section.clone()).or_default();
            } else if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim().to_string();
                let val = v.trim().to_string();
                sections.entry(current_section.clone()).or_default().insert(key, val);
            }
        }

        Self { sections }
    }

    pub fn get_value(&self, section: &str, key: &str) -> Option<&String> {
        // Case-insensitive lookup for sections and keys
        for (sec_name, sec_keys) in &self.sections {
            if sec_name.eq_ignore_ascii_case(section) {
                for (k, v) in sec_keys {
                    if k.eq_ignore_ascii_case(key) {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    pub fn set_value(&mut self, section: &str, key: &str, value: &str) {
        let sec_key = self
            .sections
            .keys()
            .find(|k| k.eq_ignore_ascii_case(section))
            .cloned()
            .unwrap_or_else(|| section.to_string());

        let sec = self.sections.entry(sec_key).or_default();
        let target_key = sec
            .keys()
            .find(|k| k.eq_ignore_ascii_case(key))
            .cloned()
            .unwrap_or_else(|| key.to_string());

        sec.insert(target_key, value.to_string());
    }

    pub fn remove_key(&mut self, section: &str, key: &str) {
        if let Some(sec_key) = self
            .sections
            .keys()
            .find(|k| k.eq_ignore_ascii_case(section))
            .cloned()
        {
            if let Some(sec) = self.sections.get_mut(&sec_key) {
                if let Some(k) = sec.keys().find(|k| k.eq_ignore_ascii_case(key)).cloned() {
                    sec.remove(&k);
                }
                if sec.is_empty() {
                    self.sections.remove(&sec_key);
                }
            }
        }
    }

    pub fn remove_section(&mut self, section: &str) {
        if let Some(sec_key) = self
            .sections
            .keys()
            .find(|k| k.eq_ignore_ascii_case(section))
            .cloned()
        {
            self.sections.remove(&sec_key);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sections.values().all(|s| s.is_empty())
    }

    pub fn to_utf16le_bytes(&self) -> Vec<u8> {
        let mut text = String::new();
        for (sec_name, sec_keys) in &self.sections {
            if sec_keys.is_empty() {
                continue;
            }
            text.push('[');
            text.push_str(sec_name);
            text.push_str("]\r\n");
            for (k, v) in sec_keys {
                text.push_str(k);
                text.push('=');
                text.push_str(v);
                text.push_str("\r\n");
            }
            text.push_str("\r\n");
        }

        let mut bytes = Vec::with_capacity(2 + text.len() * 2);
        // UTF-16 LE BOM: 0xFF, 0xFE
        bytes.push(0xFF);
        bytes.push(0xFE);
        for code_unit in text.encode_utf16() {
            bytes.extend_from_slice(&code_unit.to_le_bytes());
        }
        bytes
    }
}

/// Читает содержимое desktop.ini из заданной папки с поддержкой UTF-16LE, UTF-8 и ANSI
pub fn read_desktop_ini<P: AsRef<Path>>(folder_path: P) -> Option<IniDocument> {
    let ini_path = folder_path.as_ref().join("desktop.ini");
    if !ini_path.is_file() {
        return None;
    }

    let bytes = fs::read(&ini_path).ok()?;
    if bytes.is_empty() {
        return Some(IniDocument::default());
    }

    let content = decode_ini_bytes(&bytes);
    Some(IniDocument::parse(&content))
}

pub fn decode_ini_bytes(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        // UTF-16 LE with BOM
        let u16s: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        return String::from_utf16_lossy(&u16s);
    }
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        // UTF-8 with BOM
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    // Попытка как валидный UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }

    // На Windows декодируем через системную ANSI кодовую страницу (CP_ACP, например CP1251)
    #[cfg(windows)]
    {
        extern "system" {
            fn MultiByteToWideChar(
                CodePage: u32,
                dwFlags: u32,
                lpMultiByteStr: *const u8,
                cbMultiByte: i32,
                lpWideCharStr: *mut u16,
                cchWideChar: i32,
            ) -> i32;
        }
        const CP_ACP: u32 = 0;
        unsafe {
            let len = MultiByteToWideChar(
                CP_ACP,
                0,
                bytes.as_ptr(),
                bytes.len() as i32,
                std::ptr::null_mut(),
                0,
            );
            if len > 0 {
                let mut wide = vec![0u16; len as usize];
                let res = MultiByteToWideChar(
                    CP_ACP,
                    0,
                    bytes.as_ptr(),
                    bytes.len() as i32,
                    wide.as_mut_ptr(),
                    len,
                );
                if res > 0 {
                    return String::from_utf16_lossy(&wide);
                }
            }
        }
    }

    // Резервный fallback
    bytes.iter().map(|&b| b as char).collect()
}

/// Записывает IniDocument в desktop.ini с атрибутами HIDDEN | SYSTEM и ставит READONLY на папку
pub fn write_desktop_ini<P: AsRef<Path>>(folder_path: P, ini_doc: &IniDocument) -> Result<(), String> {
    let folder_path = folder_path.as_ref();
    let ini_path = folder_path.join("desktop.ini");

    let folder_w = to_wide_null(folder_path.to_str().unwrap_or_default());
    let ini_w = to_wide_null(ini_path.to_str().unwrap_or_default());

    if ini_doc.is_empty() {
        // Если настроек не осталось, можно удалить desktop.ini и снять ReadOnly с папки
        if ini_path.exists() {
            unsafe {
                SetFileAttributesW(PCWSTR(ini_w.as_ptr()), FILE_ATTRIBUTE_NORMAL).ok();
            }
            fs::remove_file(&ini_path).map_err(|e| e.to_string())?;
        }

        unsafe {
            let attr = GetFileAttributesW(PCWSTR(folder_w.as_ptr()));
            if attr != INVALID_FILE_ATTRIBUTES {
                let new_attr = attr & !FILE_ATTRIBUTE_READONLY.0;
                SetFileAttributesW(PCWSTR(folder_w.as_ptr()), windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(new_attr)).ok();
            }
        }

        notify_shell_folder_updated(folder_path);
        return Ok(());
    }

    // Снимаем временные атрибуты с ini для возможности перезаписи
    if ini_path.exists() {
        unsafe {
            SetFileAttributesW(PCWSTR(ini_w.as_ptr()), FILE_ATTRIBUTE_NORMAL).ok();
        }
    }

    let bytes = ini_doc.to_utf16le_bytes();
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&ini_path)
        .map_err(|e| format!("Не удалось открыть desktop.ini для записи: {e}"))?;

    file.write_all(&bytes)
        .map_err(|e| format!("Ошибка записи в desktop.ini: {e}"))?;
    file.flush().ok();
    drop(file);

    // Устанавливаем системные атрибуты на desktop.ini
    unsafe {
        SetFileAttributesW(
            PCWSTR(ini_w.as_ptr()),
            FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM,
        )
        .ok();

        // Обязательно выставляем ReadOnly на саму папку, иначе Проводник не читает desktop.ini!
        let attr = GetFileAttributesW(PCWSTR(folder_w.as_ptr()));
        if attr != INVALID_FILE_ATTRIBUTES {
            SetFileAttributesW(
                PCWSTR(folder_w.as_ptr()),
                windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(attr | FILE_ATTRIBUTE_READONLY.0),
            )
            .ok();
        }
    }

    notify_shell_folder_updated(folder_path);
    Ok(())
}

/// Получает текущее состояние папки (текущий тег и путь к иконке)
pub fn get_folder_state<P: AsRef<Path>>(folder_path: P) -> FolderState {
    let p = folder_path.as_ref();
    let mut state = FolderState {
        folder_path: p.to_path_buf(),
        tag: None,
        icon_path: None,
    };

    if let Some(ini) = read_desktop_ini(p) {
        // Сначала ищем в ProjectTag, затем в InfoTip
        state.tag = ini
            .get_value(EXPLORER_TAGS_SECTION, "ProjectTag")
            .or_else(|| ini.get_value(SHELL_CLASS_INFO, "InfoTip"))
            .cloned()
            .filter(|s| !s.is_empty());

        state.icon_path = ini
            .get_value(SHELL_CLASS_INFO, "IconResource")
            .and_then(|res| res.split(',').next().map(|s| s.to_string()))
            .or_else(|| ini.get_value(SHELL_CLASS_INFO, "IconFile").cloned())
            .filter(|s| !s.is_empty());
    }

    state
}

/// Применяет цвет (иконку) и/или тег к папке
pub fn apply_folder_settings<P: AsRef<Path>>(
    folder_path: P,
    icon_path: Option<&str>,
    tag: Option<&str>,
) -> Result<(), String> {
    let folder_path = folder_path.as_ref();
    let mut ini = read_desktop_ini(folder_path).unwrap_or_default();

    if let Some(icon) = icon_path {
        if !icon.is_empty() {
            ini.set_value(SHELL_CLASS_INFO, "IconResource", &format!("{icon},0"));
            ini.set_value(SHELL_CLASS_INFO, "IconFile", icon);
            ini.set_value(SHELL_CLASS_INFO, "IconIndex", "0");

            // Вызываем WinAPI SHFOLDERCUSTOMSETTINGS для дополнительной регистрации в Shell
            call_sh_custom_settings_icon(folder_path, icon);
        }
    }

    if let Some(t) = tag {
        if !t.is_empty() {
            ini.set_value(SHELL_CLASS_INFO, "InfoTip", t);
            ini.set_value(SUMMARY_INFO_SECTION, "Prop5", &format!("31,{t}"));
            ini.set_value(EXPLORER_TAGS_SECTION, "ProjectTag", t);
        } else {
            // Очищаем тег
            ini.remove_key(SHELL_CLASS_INFO, "InfoTip");
            ini.remove_key(SUMMARY_INFO_SECTION, "Prop5");
            ini.remove_key(EXPLORER_TAGS_SECTION, "ProjectTag");
        }
    }

    let res = write_desktop_ini(folder_path, &ini);
    if res.is_ok() {
        sync_registry_after_change(folder_path);
    }
    res
}

/// Снять только тег с папки, сохранив иконку
pub fn remove_folder_tag<P: AsRef<Path>>(folder_path: P) -> Result<(), String> {
    let folder_path = folder_path.as_ref();
    let mut ini = match read_desktop_ini(folder_path) {
        Some(doc) => doc,
        None => return Ok(()),
    };

    ini.remove_key(SHELL_CLASS_INFO, "InfoTip");
    ini.remove_key(SUMMARY_INFO_SECTION, "Prop5");
    ini.remove_key(EXPLORER_TAGS_SECTION, "ProjectTag");

    let res = write_desktop_ini(folder_path, &ini);
    if res.is_ok() {
        sync_registry_after_change(folder_path);
    }
    res
}

/// Снять только цвет (иконку), сохранив тег
pub fn remove_folder_color<P: AsRef<Path>>(folder_path: P) -> Result<(), String> {
    let folder_path = folder_path.as_ref();
    let mut ini = match read_desktop_ini(folder_path) {
        Some(doc) => doc,
        None => return Ok(()),
    };

    ini.remove_key(SHELL_CLASS_INFO, "IconResource");
    ini.remove_key(SHELL_CLASS_INFO, "IconFile");
    ini.remove_key(SHELL_CLASS_INFO, "IconIndex");

    call_sh_custom_settings_reset_icon(folder_path);
    let res = write_desktop_ini(folder_path, &ini);
    if res.is_ok() {
        sync_registry_after_change(folder_path);
    }
    res
}

/// Полный сброс папки (удаление иконки и тега)
pub fn reset_folder<P: AsRef<Path>>(folder_path: P) -> Result<(), String> {
    let folder_path = folder_path.as_ref();
    let mut ini = match read_desktop_ini(folder_path) {
        Some(doc) => doc,
        None => return Ok(()),
    };

    ini.remove_key(SHELL_CLASS_INFO, "IconResource");
    ini.remove_key(SHELL_CLASS_INFO, "IconFile");
    ini.remove_key(SHELL_CLASS_INFO, "IconIndex");
    ini.remove_key(SHELL_CLASS_INFO, "InfoTip");
    ini.remove_key(SUMMARY_INFO_SECTION, "Prop5");
    ini.remove_key(EXPLORER_TAGS_SECTION, "ProjectTag");

    call_sh_custom_settings_reset_all(folder_path);
    let res = write_desktop_ini(folder_path, &ini);
    if res.is_ok() {
        sync_registry_after_change(folder_path);
    }
    res
}

fn sync_registry_after_change(folder_path: &Path) {
    let state = get_folder_state(folder_path);
    let mut reg = crate::registry::FolderRegistry::load();
    if state.tag.is_none() && state.icon_path.is_none() {
        reg.remove(folder_path);
    } else {
        let (vol, fid) = match crate::file_id::get_folder_file_id(folder_path) {
            Some((v, id)) => (Some(v), Some(id)),
            None => (None, None),
        };
        reg.upsert(crate::registry::TaggedFolderRecord {
            path: folder_path.to_path_buf(),
            tag: state.tag,
            icon_path: state.icon_path,
            color_hex: None,
            volume_serial: vol,
            file_id: fid,
            updated_at: crate::registry::current_timestamp(),
        });
    }
    let _ = reg.save();
}

fn call_sh_custom_settings_icon(folder_path: &Path, icon_path: &str) {
    let folder_w = to_wide_null(folder_path.to_str().unwrap_or_default());
    let mut icon_w = to_wide_null(icon_path);

    let mut fcs = SHFOLDERCUSTOMSETTINGS {
        dwSize: std::mem::size_of::<SHFOLDERCUSTOMSETTINGS>() as u32,
        dwMask: FCSM_ICONFILE,
        pszIconFile: windows::core::PWSTR(icon_w.as_mut_ptr()),
        iIconIndex: 0,
        ..Default::default()
    };

    unsafe {
        SHGetSetFolderCustomSettings(
            &mut fcs,
            PCWSTR(folder_w.as_ptr()),
            FCS_FORCEWRITE,
        )
        .ok();
    }
}

fn call_sh_custom_settings_reset_icon(folder_path: &Path) {
    let folder_w = to_wide_null(folder_path.to_str().unwrap_or_default());
    let mut empty_w = to_wide_null("");

    let mut fcs = SHFOLDERCUSTOMSETTINGS {
        dwSize: std::mem::size_of::<SHFOLDERCUSTOMSETTINGS>() as u32,
        dwMask: FCSM_ICONFILE,
        pszIconFile: windows::core::PWSTR(empty_w.as_mut_ptr()),
        iIconIndex: 0,
        ..Default::default()
    };

    unsafe {
        SHGetSetFolderCustomSettings(
            &mut fcs,
            PCWSTR(folder_w.as_ptr()),
            FCS_FORCEWRITE,
        )
        .ok();
    }
}

fn call_sh_custom_settings_reset_all(folder_path: &Path) {
    let folder_w = to_wide_null(folder_path.to_str().unwrap_or_default());
    let mut empty_w = to_wide_null("");

    let mut fcs = SHFOLDERCUSTOMSETTINGS {
        dwSize: std::mem::size_of::<SHFOLDERCUSTOMSETTINGS>() as u32,
        dwMask: FCSM_ICONFILE | FCSM_INFOTIP,
        pszIconFile: windows::core::PWSTR(empty_w.as_mut_ptr()),
        pszInfoTip: windows::core::PWSTR(empty_w.as_mut_ptr()),
        iIconIndex: 0,
        ..Default::default()
    };

    unsafe {
        SHGetSetFolderCustomSettings(
            &mut fcs,
            PCWSTR(folder_w.as_ptr()),
            FCS_FORCEWRITE,
        )
        .ok();
    }
}

pub fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_ansi_cp1251_decoding() {
        // "Программы" in CP1251: CF F0 EE E3 F0 E0 EC EC FB
        let cp1251_bytes = vec![
            b'[', b'.', b'S', b'h', b'e', b'l', b'l', b'C', b'l', b'a', b's', b's', b'I', b'n', b'f', b'o', b']',
            b'\r', b'\n',
            b'I', b'n', b'f', b'o', b'T', b'i', b'p', b'=',
            0xCF, 0xF0, 0xEE, 0xE3, 0xF0, 0xE0, 0xEC, 0xEC, 0xFB,
            b'\r', b'\n',
        ];
        let decoded = decode_ini_bytes(&cp1251_bytes);
        let doc = IniDocument::parse(&decoded);
        let val = doc.get_value(".ShellClassInfo", "InfoTip").unwrap();
        assert_eq!(val, "Программы");
    }

    #[test]
    fn test_ini_parse_and_modify() {
        let content = "[.ShellClassInfo]\r\nInfoTip=OldTip\r\n";
        let mut doc = IniDocument::parse(content);
        assert_eq!(doc.get_value(".ShellClassInfo", "InfoTip").map(|s| s.as_str()), Some("OldTip"));

        doc.set_value(".ShellClassInfo", "InfoTip", "NewTip");
        doc.set_value("ExplorerTags", "ProjectTag", "🔥 Срочно");

        assert_eq!(doc.get_value(".ShellClassInfo", "InfoTip").map(|s| s.as_str()), Some("NewTip"));
        assert_eq!(doc.get_value("ExplorerTags", "ProjectTag").map(|s| s.as_str()), Some("🔥 Срочно"));

        doc.remove_key(".ShellClassInfo", "InfoTip");
        assert_eq!(doc.get_value(".ShellClassInfo", "InfoTip"), None);
    }

    #[test]
    fn test_folder_apply_and_separate_reset() {
        let dir = tempdir().unwrap();
        let folder_path = dir.path();

        // 1. Применяем и цвет, и тег
        apply_folder_settings(folder_path, Some("C:\\icons\\red.ico"), Some("🔥 Срочно")).unwrap();

        let state = get_folder_state(folder_path);
        assert_eq!(state.tag.as_deref(), Some("🔥 Срочно"));
        assert_eq!(state.icon_path.as_deref(), Some("C:\\icons\\red.ico"));

        // 2. Снимаем только тег
        remove_folder_tag(folder_path).unwrap();
        let state_after_tag_remove = get_folder_state(folder_path);
        assert_eq!(state_after_tag_remove.tag, None);
        assert_eq!(state_after_tag_remove.icon_path.as_deref(), Some("C:\\icons\\red.ico"));

        // 3. Возвращаем тег и снимаем только цвет
        apply_folder_settings(folder_path, None, Some("⭐ Важно")).unwrap();
        remove_folder_color(folder_path).unwrap();
        let state_after_color_remove = get_folder_state(folder_path);
        assert_eq!(state_after_color_remove.tag.as_deref(), Some("⭐ Важно"));
        assert_eq!(state_after_color_remove.icon_path, None);

        // 4. Полный сброс
        reset_folder(folder_path).unwrap();
        let state_reset = get_folder_state(folder_path);
        assert_eq!(state_reset.tag, None);
        assert_eq!(state_reset.icon_path, None);
    }
}

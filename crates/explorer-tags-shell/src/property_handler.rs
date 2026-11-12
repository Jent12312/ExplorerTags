use std::path::PathBuf;
use std::sync::RwLock;
use explorer_tags_core::desktop_ini::read_desktop_ini;
use explorer_tags_core::guids::{PKEY_EXPLORER_TAGS_PROJECT_TAG, PKEY_KEYWORDS};
use windows::core::{implement, PCWSTR, Result};
use windows::Win32::Foundation::{E_INVALIDARG, E_POINTER, STG_E_ACCESSDENIED};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Com::StructuredStorage::InitPropVariantFromStringVector;
use windows::Win32::UI::Shell::PropertiesSystem::{
    IInitializeWithFile_Impl, IPropertyStore, IPropertyStore_Impl, PROPERTYKEY,
};
use windows::Win32::UI::Shell::{IInitializeWithItem_Impl, IShellItem, SIGDN_FILESYSPATH};

#[implement(
    IPropertyStore,
    windows::Win32::UI::Shell::IInitializeWithItem,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile
)]
pub struct PropertyHandler {
    folder_path: RwLock<PathBuf>,
}

impl PropertyHandler {
    pub fn new() -> Self {
        crate::dll_add_ref();
        Self {
            folder_path: RwLock::new(PathBuf::new()),
        }
    }
}

impl Drop for PropertyHandler {
    fn drop(&mut self) {
        crate::dll_release();
    }
}

impl IInitializeWithItem_Impl for PropertyHandler_Impl {
    fn Initialize(&self, psi: Option<&IShellItem>, _grfmode: u32) -> Result<()> {
        let Some(item) = psi else {
            return Err(E_INVALIDARG.into());
        };

        unsafe {
            let path_pwstr = item.GetDisplayName(SIGDN_FILESYSPATH)?;
            if let Ok(path_str) = path_pwstr.to_string() {
                if let Ok(mut lock) = self.folder_path.write() {
                    *lock = PathBuf::from(path_str);
                }
            }
            CoTaskMemFree(Some(path_pwstr.0 as _));
        }

        Ok(())
    }
}

impl IInitializeWithFile_Impl for PropertyHandler_Impl {
    fn Initialize(&self, pszfilepath: &PCWSTR, _grfmode: u32) -> Result<()> {
        if pszfilepath.is_null() {
            return Err(E_INVALIDARG.into());
        }

        unsafe {
            if let Ok(path_str) = pszfilepath.to_string() {
                if let Ok(mut lock) = self.folder_path.write() {
                    *lock = PathBuf::from(path_str);
                }
            }
        }

        Ok(())
    }
}

impl IPropertyStore_Impl for PropertyHandler_Impl {
    fn GetCount(&self) -> Result<u32> {
        // Поддерживаем два свойства: Keywords (системные теги) и ExplorerTags.ProjectTag
        Ok(2)
    }

    fn GetAt(&self, iprop: u32, pkey: *mut PROPERTYKEY) -> Result<()> {
        if pkey.is_null() {
            return Err(E_POINTER.into());
        }

        unsafe {
            match iprop {
                0 => {
                    *pkey = PKEY_KEYWORDS;
                    Ok(())
                }
                1 => {
                    *pkey = PKEY_EXPLORER_TAGS_PROJECT_TAG;
                    Ok(())
                }
                _ => Err(E_INVALIDARG.into()),
            }
        }
    }

    fn GetValue(&self, key: *const PROPERTYKEY) -> Result<windows_core::PROPVARIANT> {
        if key.is_null() {
            return Err(E_POINTER.into());
        }

        let key_ref = unsafe { &*key };
        let is_custom_tag = key_ref.fmtid == PKEY_EXPLORER_TAGS_PROJECT_TAG.fmtid
            && key_ref.pid == PKEY_EXPLORER_TAGS_PROJECT_TAG.pid;
        let is_keywords = key_ref.fmtid == PKEY_KEYWORDS.fmtid && key_ref.pid == PKEY_KEYWORDS.pid;

        if !is_custom_tag && !is_keywords {
            return Ok(windows_core::PROPVARIANT::default());
        }

        let folder = match self.folder_path.read() {
            Ok(lock) => lock.clone(),
            Err(_) => return Ok(windows_core::PROPVARIANT::default()),
        };

        if folder.as_os_str().is_empty() {
            return Ok(windows_core::PROPVARIANT::default());
        }

        if let Some(ini) = read_desktop_ini(&folder) {
            let tag = ini
                .get_value("ExplorerTags", "ProjectTag")
                .or_else(|| ini.get_value(".ShellClassInfo", "InfoTip"))
                .cloned();

            if let Some(tag_str) = tag {
                if !tag_str.is_empty() {
                    let wide: Vec<u16> = tag_str.encode_utf16().chain(std::iter::once(0)).collect();

                    unsafe {
                        if is_custom_tag {
                            // ExplorerTags.ProjectTag (тип String в схеме)
                            return Ok(windows_core::PROPVARIANT::from(tag_str.as_str()));
                        } else if is_keywords {
                            // PKEY_Keywords в Windows является вектором строк (VT_VECTOR | VT_LPWSTR)
                            return InitPropVariantFromStringVector(Some(&[PCWSTR(wide.as_ptr())]));
                        }
                    }
                }
            }
        }

        Ok(windows_core::PROPVARIANT::default())
    }

    fn SetValue(&self, _key: *const PROPERTYKEY, _propvar: *const windows_core::PROPVARIANT) -> Result<()> {
        Err(STG_E_ACCESSDENIED.into())
    }

    fn Commit(&self) -> Result<()> {
        Ok(())
    }
}

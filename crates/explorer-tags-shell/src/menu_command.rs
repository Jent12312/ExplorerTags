use std::path::PathBuf;
use windows::core::{implement, PCWSTR, PWSTR, Result, GUID};
use windows::Win32::Foundation::{BOOL, E_INVALIDARG, E_OUTOFMEMORY};
use windows::Win32::System::Com::{CoTaskMemAlloc, CoTaskMemFree, IBindCtx};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Threading::{
    CreateProcessW, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
};
use windows::Win32::UI::Shell::{
    IEnumExplorerCommand, IExplorerCommand, IExplorerCommand_Impl, IShellItemArray,
    SIGDN_FILESYSPATH,
};

const ECS_ENABLED: u32 = 0;
const ECF_DEFAULT: u32 = 0;

#[implement(IExplorerCommand)]
pub struct MainMenuCommand;

impl MainMenuCommand {
    pub fn new() -> Self {
        crate::dll_add_ref();
        Self
    }
}

impl Drop for MainMenuCommand {
    fn drop(&mut self) {
        crate::dll_release();
    }
}

fn alloc_pwstr(s: &str) -> Result<PWSTR> {
    let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let bytes_len = wide.len() * std::mem::size_of::<u16>();
        let mem = CoTaskMemAlloc(bytes_len) as *mut u16;
        if mem.is_null() {
            return Err(E_OUTOFMEMORY.into());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), mem, wide.len());
        Ok(PWSTR(mem))
    }
}

impl IExplorerCommand_Impl for MainMenuCommand_Impl {
    fn GetTitle(&self, _psiitemarray: Option<&IShellItemArray>) -> Result<PWSTR> {
        alloc_pwstr("Настроить папку...")
    }

    fn GetIcon(&self, _psiitemarray: Option<&IShellItemArray>) -> Result<PWSTR> {
        if let Some(ico) = find_menu_icon_path() {
            return alloc_pwstr(&ico.to_string_lossy());
        }
        alloc_pwstr("shell32.dll,3")
    }

    fn GetToolTip(&self, _psiitemarray: Option<&IShellItemArray>) -> Result<PWSTR> {
        alloc_pwstr("Изменить цвет и тег")
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Ok(GUID::zeroed())
    }

    fn GetState(&self, _psiitemarray: Option<&IShellItemArray>, _foktobeslow: BOOL) -> Result<u32> {
        Ok(ECS_ENABLED)
    }

    fn GetFlags(&self) -> Result<u32> {
        Ok(ECF_DEFAULT)
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        // Одиночный пункт меню без выпадающих подпунктов
        Err(windows::Win32::Foundation::E_NOTIMPL.into())
    }

    fn Invoke(
        &self,
        psiitemarray: Option<&IShellItemArray>,
        _pbc: Option<&IBindCtx>,
    ) -> Result<()> {
        let Some(items) = psiitemarray else {
            return Err(E_INVALIDARG.into());
        };

        let count = unsafe { items.GetCount()? };
        let mut paths = Vec::new();

        for i in 0..count {
            unsafe {
                if let Ok(item) = items.GetItemAt(i) {
                    if let Ok(name_pwstr) = item.GetDisplayName(SIGDN_FILESYSPATH) {
                        if let Ok(path_str) = name_pwstr.to_string() {
                            paths.push(path_str);
                        }
                        CoTaskMemFree(Some(name_pwstr.0 as _));
                    }
                }
            }
        }

        if paths.is_empty() {
            return Ok(());
        }

        let exe_path = find_ui_executable();
        if !exe_path.exists() {
            return Ok(());
        }

        let mut cmd_line = format!("\"{}\"", exe_path.to_string_lossy());
        for p in &paths {
            cmd_line.push(' ');
            cmd_line.push('"');
            cmd_line.push_str(p);
            cmd_line.push('"');
        }

        let mut cmd_wide: Vec<u16> = cmd_line.encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            let si = STARTUPINFOW {
                cb: std::mem::size_of::<STARTUPINFOW>() as u32,
                ..Default::default()
            };
            let mut pi = PROCESS_INFORMATION::default();

            let success = CreateProcessW(
                PCWSTR::null(),
                PWSTR(cmd_wide.as_mut_ptr()),
                None,
                None,
                false,
                PROCESS_CREATION_FLAGS(0),
                None,
                PCWSTR::null(),
                &si,
                &mut pi,
            );

            if success.is_ok() {
                windows::Win32::Foundation::CloseHandle(pi.hProcess).ok();
                windows::Win32::Foundation::CloseHandle(pi.hThread).ok();
            }
        }

        Ok(())
    }
}

fn find_ui_executable() -> PathBuf {
    unsafe {
        let mut buf = [0u16; 1024];
        let len = GetModuleFileNameW(crate::get_hinstance(), &mut buf);
        if len > 0 {
            let dll_path = String::from_utf16_lossy(&buf[..len as usize]);
            let p = PathBuf::from(dll_path);
            if let Some(parent) = p.parent() {
                let candidate = parent.join("ExplorerTagsUI.exe");
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }

    if let Ok(app_data) = std::env::var("APPDATA") {
        let candidate = PathBuf::from(app_data)
            .join("ExplorerTags")
            .join("ExplorerTagsUI.exe");
        if candidate.is_file() {
            return candidate;
        }
    }

    PathBuf::from("ExplorerTagsUI.exe")
}

pub fn find_menu_icon_path() -> Option<PathBuf> {
    unsafe {
        let mut buf = [0u16; 1024];
        let len = GetModuleFileNameW(crate::get_hinstance(), &mut buf);
        if len > 0 {
            let dll_path = String::from_utf16_lossy(&buf[..len as usize]);
            let p = PathBuf::from(&dll_path);
            if let Some(parent) = p.parent() {
                let ico_path = parent.join("Icons").join("Rust 1 Folder.ico");
                if ico_path.is_file() {
                    return Some(ico_path);
                }
            }
        }
    }

    if let Ok(app_data) = std::env::var("APPDATA") {
        let ico_path = PathBuf::from(app_data)
            .join("ExplorerTags")
            .join("Icons")
            .join("Rust 1 Folder.ico");
        if ico_path.is_file() {
            return Some(ico_path);
        }
    }

    let dev_ico = PathBuf::from("B:\\ExplorerTags\\Icons\\Rust 1 Folder.ico");
    if dev_ico.is_file() {
        return Some(dev_ico);
    }

    None
}

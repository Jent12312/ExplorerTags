use std::path::PathBuf;
use windows::core::{Interface, VARIANT};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, IDispatch, CLSCTX_ALL};
use windows::Win32::UI::Shell::{
    IShellFolderViewDual, IShellWindows, ShellWindowFindWindowOptions, ShellWindowTypeConstants,
    ShellWindows,
};
use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GetClassNameW, GetForegroundWindow, GA_ROOT};

const SWC_DESKTOP: i32 = 8;
const SWFO_NEEDDISPATCH: i32 = 1;

fn log_debug(msg: &str) {
    if let Ok(app_data) = std::env::var("APPDATA") {
        let p = std::path::PathBuf::from(app_data).join("ExplorerTags").join("quicklook.log");
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .map(|mut f| {
                use std::io::Write;
                writeln!(f, "[{:?}] {}", std::time::SystemTime::now(), msg).ok();
            });
    }
}

pub fn is_explorer_window(hwnd: HWND) -> bool {
    unsafe {
        let mut pid = 0u32;
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != 0 {
            if let Ok(process) = windows::Win32::System::Threading::OpenProcess(
                windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                pid,
            ) {
                let mut name_buf = [0u16; 260];
                let mut size = name_buf.len() as u32;
                if windows::Win32::System::Threading::QueryFullProcessImageNameW(
                    process,
                    windows::Win32::System::Threading::PROCESS_NAME_FORMAT(0),
                    windows::core::PWSTR(name_buf.as_mut_ptr()),
                    &mut size,
                ).is_ok() {
                    let name = String::from_utf16_lossy(&name_buf[..size as usize]);
                    let _ = windows::Win32::Foundation::CloseHandle(process);
                    if name.to_lowercase().ends_with("explorer.exe") {
                        return true;
                    }
                } else {
                    let _ = windows::Win32::Foundation::CloseHandle(process);
                }
            }
        }
    }

    if check_class_matches(hwnd) {
        return true;
    }
    unsafe {
        let root = GetAncestor(hwnd, GA_ROOT);
        if !root.0.is_null() && root != hwnd && check_class_matches(root) {
            return true;
        }
    }
    false
}

fn check_class_matches(hwnd: HWND) -> bool {
    let mut class_buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut class_buf) };
    if len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
        return class_name == "CabinetWClass"
            || class_name == "ExploreWClass"
            || class_name == "WorkerW"
            || class_name == "Progman"
            || class_name == "ShellTabWindowClass"
            || class_name.contains("Explorer");
    }
    false
}

pub fn is_desktop_window(hwnd: HWND) -> bool {
    let mut class_buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut class_buf) };
    if len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
        if class_name == "WorkerW" || class_name == "Progman" {
            return true;
        }
    }
    unsafe {
        let root = GetAncestor(hwnd, GA_ROOT);
        if !root.0.is_null() && root != hwnd {
            let len = GetClassNameW(root, &mut class_buf);
            if len > 0 {
                let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
                return class_name == "WorkerW" || class_name == "Progman";
            }
        }
    }
    false
}

/// Получает путь к текущему выделенному файлу в активном окне Проводника или на рабочем столе
pub fn get_explorer_selected_path() -> Option<PathBuf> {
    let foreground = unsafe { GetForegroundWindow() };
    log_debug(&format!("get_explorer_selected_path: fg={:?}", foreground.0));
    if foreground.0.is_null() {
        return None;
    }

    unsafe {
        let shell_windows: IShellWindows = match CoCreateInstance(&ShellWindows, None, CLSCTX_ALL) {
            Ok(sw) => sw,
            Err(e) => {
                log_debug(&format!("CoCreateInstance(ShellWindows) error: {:?}", e));
                return None;
            }
        };

        // 1. Проверяем рабочий стол если фокус на Desktop
        if is_desktop_window(foreground) {
            log_debug("Desktop foreground window detected");
            let mut dummy = 0i32;
            if let Ok(dispatch) = shell_windows.FindWindowSW(
                &VARIANT::default(),
                &VARIANT::default(),
                ShellWindowTypeConstants(SWC_DESKTOP),
                &mut dummy,
                ShellWindowFindWindowOptions(SWFO_NEEDDISPATCH),
            ) {
                if let Some(p) = extract_path_from_browser(&dispatch) {
                    log_debug(&format!("Desktop selected: {:?}", p));
                    return Some(p);
                }
            }
        }

        // 2. Ищем среди открытых окон и вкладок Проводника
        let count = shell_windows.Count().ok().unwrap_or(0);
        log_debug(&format!("shell_windows.Count() = {}", count));
        let mut fallback_path: Option<PathBuf> = None;

        for i in 0..count {
            let variant_index = VARIANT::from(i);
            let Ok(dispatch) = shell_windows.Item(&variant_index) else {
                continue;
            };

            let Ok(browser) = dispatch.cast::<windows::Win32::UI::Shell::IWebBrowser2>() else {
                continue;
            };

            let Ok(browser_hwnd_ptr) = browser.HWND() else {
                continue;
            };
            let browser_hwnd = HWND(browser_hwnd_ptr.0 as *mut _);

            let is_match = browser_hwnd == foreground || is_same_window_tree(browser_hwnd, foreground);

            if let Some(p) = extract_path_from_browser(&dispatch) {
                log_debug(&format!("Window {} (is_match={}) selected: {:?}", i, is_match, p));
                if is_match {
                    return Some(p);
                }
                if fallback_path.is_none() {
                    fallback_path = Some(p);
                }
            }
        }

        if fallback_path.is_some() {
            log_debug(&format!("Returning fallback_path: {:?}", fallback_path));
            return fallback_path;
        }
    }

    log_debug("No path extracted");
    None
}

unsafe fn extract_path_from_browser(dispatch: &IDispatch) -> Option<PathBuf> {
    let browser = dispatch.cast::<windows::Win32::UI::Shell::IWebBrowser2>().ok()?;
    let doc_dispatch = browser.Document().ok()?;
    let folder_view = doc_dispatch.cast::<IShellFolderViewDual>().ok()?;
    let selected_items = folder_view.SelectedItems().ok()?;
    let selected_count = selected_items.Count().ok().unwrap_or(0);

    if selected_count > 0 {
        let first_idx = VARIANT::from(0i32);
        let item = selected_items.Item(&first_idx).ok()?;
        let bstr_path = item.Path().ok()?;
        let path_str = bstr_path.to_string();
        if !path_str.is_empty() {
            let pb = PathBuf::from(path_str);
            if pb.exists() {
                return Some(pb);
            }
        }
    }

    None
}

fn is_same_window_tree(hwnd1: HWND, hwnd2: HWND) -> bool {
    if hwnd1 == hwnd2 {
        return true;
    }
    unsafe {
        let root1 = GetAncestor(hwnd1, GA_ROOT);
        let root2 = GetAncestor(hwnd2, GA_ROOT);
        let r1 = if root1.0.is_null() { hwnd1 } else { root1 };
        let r2 = if root2.0.is_null() { hwnd2 } else { root2 };
        r1 == r2
    }
}

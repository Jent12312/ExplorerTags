use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_SPACE};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GUITHREADINFO,
    HHOOK, KBDLLHOOKSTRUCT, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WM_KEYDOWN,
};
use super::selection::{get_explorer_selected_path, is_explorer_window};

static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);
static QUICKLOOK_ENABLED: AtomicBool = AtomicBool::new(true);
static IS_QUICKLOOK_OPEN: AtomicBool = AtomicBool::new(false);
static CURRENT_PATH: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

type TriggerCallback = Box<dyn Fn(std::path::PathBuf) + Send + Sync + 'static>;
static ON_TRIGGER: std::sync::Mutex<Option<TriggerCallback>> = std::sync::Mutex::new(None);

type CloseCallback = Box<dyn Fn() + Send + Sync + 'static>;
static ON_CLOSE: std::sync::Mutex<Option<CloseCallback>> = std::sync::Mutex::new(None);

pub fn set_quicklook_enabled(enabled: bool) {
    QUICKLOOK_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn is_quicklook_enabled() -> bool {
    QUICKLOOK_ENABLED.load(Ordering::SeqCst)
}

pub fn set_quicklook_open(open: bool) {
    IS_QUICKLOOK_OPEN.store(open, Ordering::SeqCst);
    if !open {
        if let Ok(mut lock) = CURRENT_PATH.lock() {
            *lock = None;
        }
    }
}

pub fn is_quicklook_open() -> bool {
    IS_QUICKLOOK_OPEN.load(Ordering::SeqCst)
}

pub fn set_on_trigger<F>(callback: F)
where
    F: Fn(std::path::PathBuf) + Send + Sync + 'static,
{
    let mut lock = ON_TRIGGER.lock().unwrap();
    *lock = Some(Box::new(callback));
}

pub fn set_on_close<F>(callback: F)
where
    F: Fn() + Send + Sync + 'static,
{
    let mut lock = ON_CLOSE.lock().unwrap();
    *lock = Some(Box::new(callback));
}

pub fn start_keyboard_hook() -> Result<(), String> {
    let hook = unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(ll_keyboard_proc),
            None,
            0,
        )
    }
    .map_err(|e| e.to_string())?;

    HOOK_HANDLE.store(hook.0 as isize, Ordering::SeqCst);
    Ok(())
}

pub fn stop_keyboard_hook() {
    let prev = HOOK_HANDLE.swap(0, Ordering::SeqCst);
    if prev != 0 {
        unsafe {
            let _ = UnhookWindowsHookEx(HHOOK(prev as *mut _));
        }
    }
}

unsafe extern "system" fn ll_keyboard_proc(
    ncode: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if ncode >= 0 && wparam.0 as u32 == WM_KEYDOWN && is_quicklook_enabled() {
        let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);

        if kbd.vkCode == VK_ESCAPE.0 as u32 && is_quicklook_open() {
            set_quicklook_open(false);
            *CURRENT_PATH.lock().unwrap() = None;
            let lock = ON_CLOSE.lock().unwrap();
            if let Some(cb) = lock.as_ref() {
                cb();
            }
            return LRESULT(1);
        } else if kbd.vkCode == VK_SPACE.0 as u32 {
            let fg = GetForegroundWindow();
            let is_exp = !fg.0.is_null() && is_explorer_window(fg);
            let is_edit = !fg.0.is_null() && is_user_editing_text(fg);
            log_debug(&format!("Space pressed: fg={:?}, is_exp={}, is_edit={}", fg.0, is_exp, is_edit));

            if is_quicklook_open() && !is_exp {
                // Если активно само окно QuickLook и нажат Пробел - закрываем его
                log_debug("Space pressed while QuickLook window is active -> closing");
                set_quicklook_open(false);
                *CURRENT_PATH.lock().unwrap() = None;
                let lock = ON_CLOSE.lock().unwrap();
                if let Some(cb) = lock.as_ref() {
                    cb();
                }
                return LRESULT(1);
            }

            if is_exp && !is_edit {
                // Запускаем опрос выделенного файла в фоновом потоке,
                // чтобы не блокировать input-synchronous hook callback и не вызывать RPC_E_CANTCALLOUT_ININPUTSYNCCALL
                std::thread::spawn(move || {
                    unsafe {
                        let _ = windows::Win32::System::Com::CoInitializeEx(
                            None,
                            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
                        );
                    }

                    log_debug("Worker thread querying selected file in Explorer...");
                    if let Some(selected_path) = get_explorer_selected_path() {
                        log_debug(&format!("Worker thread selected: {:?}", selected_path));
                        let curr = CURRENT_PATH.lock().unwrap().clone();
                        if is_quicklook_open() && curr.as_ref() == Some(&selected_path) {
                            // Повторный пробел на том же файле закрывает предпросмотр (toggle)
                            log_debug("Toggle close on same file");
                            set_quicklook_open(false);
                            *CURRENT_PATH.lock().unwrap() = None;
                            let lock = ON_CLOSE.lock().unwrap();
                            if let Some(cb) = lock.as_ref() {
                                cb();
                            }
                        } else {
                            // Открываем или переключаем на новый файл
                            log_debug("Triggering preview callback");
                            set_quicklook_open(true);
                            *CURRENT_PATH.lock().unwrap() = Some(selected_path.clone());
                            let lock = ON_TRIGGER.lock().unwrap();
                            if let Some(cb) = lock.as_ref() {
                                cb(selected_path);
                            }
                        }
                    } else if is_quicklook_open() {
                        log_debug("No file selected while QuickLook is open -> closing");
                        set_quicklook_open(false);
                        *CURRENT_PATH.lock().unwrap() = None;
                        let lock = ON_CLOSE.lock().unwrap();
                        if let Some(cb) = lock.as_ref() {
                            cb();
                        }
                    } else {
                        log_debug("get_explorer_selected_path returned None");
                    }

                    unsafe {
                        windows::Win32::System::Com::CoUninitialize();
                    }
                });

                return LRESULT(1); // Поглощаем нажатие Space в Проводнике
            }
        }
    }

    CallNextHookEx(None, ncode, wparam, lparam)
}

fn log_debug(msg: &str) {
    if let Ok(app_data) = std::env::var("APPDATA") {
        let p = std::path::PathBuf::from(app_data).join("ExplorerTags").join("quicklook.log");
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .map(|mut f| {
                use std::io::Write;
                writeln!(f, "{}", msg).ok();
            });
    }
}

fn is_user_editing_text(fg: HWND) -> bool {
    unsafe {
        let mut gui_info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };

        let thread_id = windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(fg, None);
        if GetGUIThreadInfo(thread_id, &mut gui_info).is_ok() {
            if !gui_info.hwndFocus.0.is_null() {
                let mut class_buf = [0u16; 64];
                let len = GetClassNameW(gui_info.hwndFocus, &mut class_buf);
                if len > 0 {
                    let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
                    if class_name.eq_ignore_ascii_case("Edit") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

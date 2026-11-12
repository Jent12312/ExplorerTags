use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIM_ADD, NIM_DELETE, NIF_ICON, NIF_MESSAGE, NIF_TIP,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, GetCursorPos, GetSystemMetrics, LoadIconW, RegisterClassW,
    RegisterWindowMessageW, SetForegroundWindow, TrackPopupMenuEx, HICON, HMENU,
    IDI_APPLICATION, MF_SEPARATOR, MF_STRING, SM_CXSMICON, SM_CYSMICON,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WNDCLASSW,
    WS_OVERLAPPED, WM_USER, WM_LBUTTONUP, WM_RBUTTONUP,
};

pub const WM_TRAY_CALLBACK: u32 = WM_USER + 200;
const TRAY_ID: u32 = 1;

struct SafeNid(NOTIFYICONDATAW);
unsafe impl Send for SafeNid {}
unsafe impl Sync for SafeNid {}
static ACTIVE_NID: Mutex<Option<SafeNid>> = Mutex::new(None);

static WM_TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

type TrayActionCallback = Box<dyn Fn(u32) + Send + Sync + 'static>;
static TRAY_ACTION_CB: Mutex<Option<TrayActionCallback>> = Mutex::new(None);

pub fn set_tray_action_callback<F>(cb: F)
where
    F: Fn(u32) + Send + Sync + 'static,
{
    let mut lock = TRAY_ACTION_CB.lock().unwrap();
    *lock = Some(Box::new(cb));
}

pub struct TrayIcon {
    pub hwnd: HWND,
    nid: NOTIFYICONDATAW,
}

impl TrayIcon {
    pub fn new(icon_path: Option<&PathBuf>) -> Result<Self, String> {
        let class_name: Vec<u16> = "ExplorerTagsTrayWndClass\0".encode_utf16().collect();
        let hinstance = unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap_or_default() };

        let wc = WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        unsafe {
            let _ = RegisterClassW(&wc);
        }

        let hwnd = unsafe {
            CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(class_name.as_ptr()),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                HWND::default(),
                HMENU::default(),
                hinstance,
                None,
            )
        }
        .map_err(|e| e.to_string())?;

        let hicon = if let Some(path) = icon_path {
            load_hicon_from_file(path).unwrap_or_else(|| unsafe { LoadIconW(None, IDI_APPLICATION).unwrap() })
        } else {
            unsafe { LoadIconW(None, IDI_APPLICATION).unwrap() }
        };

        let mut tip = [0u16; 128];
        let tip_text: Vec<u16> = "ExplorerTags — Управление тегами\0".encode_utf16().collect();
        for (i, &ch) in tip_text.iter().take(127).enumerate() {
            tip[i] = ch;
        }

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAY_CALLBACK,
            hIcon: hicon,
            szTip: tip,
            ..Default::default()
        };

        let taskbar_created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        WM_TASKBAR_CREATED.store(taskbar_created, Ordering::SeqCst);

        // Пытаемся добавить иконку с несколькими попытками
        for _ in 0..5 {
            let added = unsafe { Shell_NotifyIconW(NIM_ADD, &mut nid).as_bool() };
            if added {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        if let Ok(mut lock) = ACTIVE_NID.lock() {
            *lock = Some(SafeNid(nid));
        }

        Ok(Self { hwnd, nid })
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        if let Ok(mut lock) = ACTIVE_NID.lock() {
            *lock = None;
        }
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &mut self.nid);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_msg = WM_TASKBAR_CREATED.load(Ordering::SeqCst);
    if taskbar_msg != 0 && msg == taskbar_msg {
        if let Ok(mut lock) = ACTIVE_NID.lock() {
            if let Some(safe) = lock.as_mut() {
                unsafe {
                    let _ = Shell_NotifyIconW(NIM_ADD, &mut safe.0);
                }
            }
        }
        return LRESULT(0);
    }

    if msg == WM_TRAY_CALLBACK {
        let event = lparam.0 as u32;
        if event == WM_LBUTTONUP {
            let lock = TRAY_ACTION_CB.lock().unwrap();
            if let Some(cb) = lock.as_ref() {
                cb(1001); // 1001 = Открыть настройки
            }
        } else if event == WM_RBUTTONUP {
            let mut pt = POINT::default();
            GetCursorPos(&mut pt).ok();
            let _ = SetForegroundWindow(hwnd);

            let hmenu = CreatePopupMenu().unwrap_or(HMENU::default());
            let opt1: Vec<u16> = "⚙️ Параметры ExplorerTags\0".encode_utf16().collect();
            let opt2: Vec<u16> = "📂 Открыть папку с иконками\0".encode_utf16().collect();
            let opt3: Vec<u16> = "❌ Выход\0".encode_utf16().collect();

            AppendMenuW(hmenu, MF_STRING, 1001, PCWSTR(opt1.as_ptr())).ok();
            AppendMenuW(hmenu, MF_STRING, 1002, PCWSTR(opt2.as_ptr())).ok();
            AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null()).ok();
            AppendMenuW(hmenu, MF_STRING, 1003, PCWSTR(opt3.as_ptr())).ok();

            let cmd = TrackPopupMenuEx(
                hmenu,
                (TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY).0,
                pt.x,
                pt.y,
                hwnd,
                None,
            );

            DestroyMenu(hmenu).ok();

            if cmd.0 > 0 {
                let lock = TRAY_ACTION_CB.lock().unwrap();
                if let Some(cb) = lock.as_ref() {
                    cb(cmd.0 as u32);
                }
            }
        }
        return LRESULT(0);
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn load_hicon_from_file(path: &PathBuf) -> Option<HICON> {
    use windows::Win32::UI::WindowsAndMessaging::{
        LoadImageW, IMAGE_ICON, LR_LOADFROMFILE,
    };

    let cx = unsafe { GetSystemMetrics(SM_CXSMICON) };
    let cy = unsafe { GetSystemMetrics(SM_CYSMICON) };

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let res = unsafe {
        LoadImageW(
            None,
            PCWSTR(wide.as_ptr()),
            IMAGE_ICON,
            cx,
            cy,
            LR_LOADFROMFILE,
        )
    };

    res.ok().map(|h| HICON(h.0))
}

use std::os::windows::ffi::OsStrExt;

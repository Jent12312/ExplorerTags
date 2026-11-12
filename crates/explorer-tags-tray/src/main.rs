#![windows_subsystem = "windows"]

slint::include_modules!();

mod autostart;
mod quick_look;
mod tray;

use autostart::{is_autostart_enabled, set_autostart_enabled};
use explorer_tags_core::{
    get_or_create_tinted_icon, reset_folder, FolderRegistry, TagsStore,
};
use quick_look::keyboard_hook::{
    is_quicklook_enabled, set_on_close, set_on_trigger, set_quicklook_enabled,
    set_quicklook_open, start_keyboard_hook, stop_keyboard_hook,
};
use slint::{ComponentHandle, ModelRc, SharedPixelBuffer, VecModel};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tray::{set_tray_action_callback, TrayIcon};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

fn log_app(msg: &str) {
    if let Ok(app_data) = std::env::var("APPDATA") {
        let p = std::path::PathBuf::from(app_data).join("ExplorerTags").join("tray_debug.log");
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .map(|mut f| {
                use std::io::Write;
                writeln!(f, "[{}] {}", chrono_or_simple(), msg).ok();
            });
    }
}

fn chrono_or_simple() -> String {
    format!("{:?}", std::time::SystemTime::now())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::panic::set_hook(Box::new(|info| {
        log_app(&format!("PANIC: {}", info));
    }));

    log_app("--- Starting explorer-tags-tray ---");
    // Софтверный рендерер обязателен для фонового tray-приложения без активного GPU swapchain
    std::env::set_var("SLINT_BACKEND", "winit-software");

    // 1. Инициализируем COM
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    // 2. Создаем окна настроек и быстрого предпросмотра
    log_app("Creating SettingsWindow and QuickLookWindow");
    let settings_win = match SettingsWindow::new() {
        Ok(w) => {
            log_app("SettingsWindow created successfully");
            w
        }
        Err(e) => {
            log_app(&format!("Failed to create SettingsWindow: {:?}", e));
            return Err(Box::new(e));
        }
    };
    let quicklook_win = match QuickLookWindow::new() {
        Ok(w) => {
            log_app("QuickLookWindow created successfully");
            w
        }
        Err(e) => {
            log_app(&format!("Failed to create QuickLookWindow: {:?}", e));
            return Err(Box::new(e));
        }
    };

    // 3. Загружаем иконку для системного трея
    log_app("Creating TrayIcon");
    let icons_dir = find_icons_directory();
    let tray_icon_file = icons_dir.join("Blue 1 Folder.ico");
    let _tray_icon = match TrayIcon::new(if tray_icon_file.is_file() {
        Some(&tray_icon_file)
    } else {
        None
    }) {
        Ok(t) => {
            log_app("TrayIcon created successfully");
            Some(t)
        }
        Err(e) => {
            log_app(&format!("TrayIcon creation warning: {}", e));
            None
        }
    };

    // 4. Инициализируем реестр папок и хранилище тегов
    let registry_rc = Arc::new(Mutex::new(FolderRegistry::load()));
    let tags_store_rc = Arc::new(Mutex::new(TagsStore::load()));

    refresh_folders_in_settings(&settings_win, &registry_rc.lock().unwrap());
    refresh_tags_in_settings(&settings_win, &tags_store_rc.lock().unwrap().tags());
    settings_win.set_autostart_enabled(is_autostart_enabled());
    settings_win.set_quicklook_enabled(is_quicklook_enabled());

    // 5. Настраиваем QuickLook окно и перехватчик клавиш
    let ql_weak = quicklook_win.as_weak();
    let ql_weak_for_close = quicklook_win.as_weak();

    quicklook_win.on_close_clicked(move || {
        set_quicklook_open(false);
        if let Some(ql) = ql_weak_for_close.upgrade() {
            ql.hide().ok();
        }
    });

    let ql_weak_on_close = quicklook_win.as_weak();
    set_on_close(move || {
        let ql_weak_inner = ql_weak_on_close.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ql) = ql_weak_inner.upgrade() {
                ql.hide().ok();
            }
        });
    });

    set_on_trigger(move |path| {
        let path_clone = path.clone();
        let ql_weak_inner = ql_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ql) = ql_weak_inner.upgrade() {
                let preview = quick_look::load_file_preview(&path_clone);
                ql.set_file_name(preview.file_name.into());
                ql.set_file_size(preview.file_size.into());
                ql.set_file_type(preview.file_type.into());
                ql.set_is_image(preview.is_image);
                ql.set_is_text(preview.is_text);
                if let Some(img) = preview.image {
                    ql.set_image_preview(img);
                }
                ql.set_text_content(preview.text.into());
                ql.show().ok();
            }
        });
    });

    log_app("Starting keyboard hook");
    if let Err(e) = start_keyboard_hook() {
        log_app(&format!("Failed to start keyboard hook: {}", e));
    } else {
        log_app("Keyboard hook started successfully");
    }

    // 6. Настраиваем обработчики окна настроек
    let reg_clone = registry_rc.clone();
    let set_weak = settings_win.as_weak();
    settings_win.on_untag_folder_clicked(move |folder_path| {
        let p = PathBuf::from(folder_path.as_str());
        reset_folder(&p).ok();
        let mut reg = reg_clone.lock().unwrap();
        reg.remove(&p);
        let _ = reg.save();
        if let Some(ui) = set_weak.upgrade() {
            refresh_folders_in_settings(&ui, &reg);
        }
    });

    settings_win.on_open_folder_clicked(|folder_path| {
        let _ = std::process::Command::new("explorer").arg(folder_path.as_str()).spawn();
    });

    let tags_clone = tags_store_rc.clone();
    let set_weak_tags = settings_win.as_weak();
    settings_win.on_add_tag_clicked(move |new_tag| {
        let mut store = tags_clone.lock().unwrap();
        if store.add_tag(new_tag.as_str()) {
            if let Some(ui) = set_weak_tags.upgrade() {
                refresh_tags_in_settings(&ui, store.tags());
            }
        }
    });

    let tags_clone2 = tags_store_rc.clone();
    let set_weak_tags2 = settings_win.as_weak();
    settings_win.on_delete_tag_clicked(move |tag_to_remove| {
        let mut store = tags_clone2.lock().unwrap();
        if store.remove_tag(tag_to_remove.as_str()) {
            if let Some(ui) = set_weak_tags2.upgrade() {
                refresh_tags_in_settings(&ui, store.tags());
            }
        }
    });

    settings_win.on_autostart_toggled(|enabled| {
        let _ = set_autostart_enabled(enabled);
    });

    settings_win.on_quicklook_toggled(|enabled| {
        set_quicklook_enabled(enabled);
    });

    settings_win.on_clear_icon_cache_clicked(|| {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let cache_dir = PathBuf::from(app_data).join("ExplorerTags").join("CustomIcons");
            if cache_dir.is_dir() {
                let _ = std::fs::remove_dir_all(&cache_dir);
                let _ = std::fs::create_dir_all(&cache_dir);
            }
        }
    });

    let set_weak_test = settings_win.as_weak();
    settings_win.on_generate_test_color(move |hex| {
        if let Some(rgb) = parse_hex_color(hex.as_str()) {
            if let Ok(icon_path) = get_or_create_tinted_icon(rgb) {
                if let Some(img) = load_ico_as_slint_image(&icon_path) {
                    if let Some(ui) = set_weak_test.upgrade() {
                        ui.set_test_icon_preview(img);
                    }
                }
            }
        }
    });

    // 7. Обработка действий из системного трея
    let set_weak_tray = settings_win.as_weak();
    let reg_for_show = registry_rc.clone();

    set_tray_action_callback(move |cmd| {
        match cmd {
            1001 => {
                let set_weak_inner = set_weak_tray.clone();
                let reg_inner = reg_for_show.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = set_weak_inner.upgrade() {
                        let mut reg = reg_inner.lock().unwrap();
                        *reg = FolderRegistry::load();
                        refresh_folders_in_settings(&ui, &reg);
                        ui.show().ok();
                    }
                });
            }
            1002 => {
                let icons = find_icons_directory();
                let _ = std::process::Command::new("explorer").arg(icons).spawn();
            }
            1003 => {
                let _ = slint::invoke_from_event_loop(|| {
                    stop_keyboard_hook();
                    let _ = slint::quit_event_loop();
                });
            }
            _ => {}
        }
    });

    // 8. Запуск цикла обработки событий Slint (фоновый режим, не выходит при закрытых окнах)
    log_app("Entering slint::run_event_loop_until_quit");
    let res = slint::run_event_loop_until_quit();
    log_app(&format!("slint::run_event_loop_until_quit returned: {:?}", res));
    stop_keyboard_hook();

    res.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

fn refresh_folders_in_settings(ui: &SettingsWindow, registry: &FolderRegistry) {
    let mut entries = Vec::new();
    for r in registry.all() {
        let date_str = if r.updated_at > 0 {
            format!("Изменено: {}", r.updated_at)
        } else {
            "".to_string()
        };
        entries.push(FolderEntry {
            path: r.path.to_string_lossy().to_string().into(),
            tag: r.tag.clone().unwrap_or_default().into(),
            updated_at: date_str.into(),
        });
    }
    ui.set_folders_list(ModelRc::new(VecModel::from(entries)));
}

fn refresh_tags_in_settings(ui: &SettingsWindow, tags: &[String]) {
    let slint_strings: Vec<slint::SharedString> =
        tags.iter().map(|s| slint::SharedString::from(s.as_str())).collect();
    ui.set_tags_list(ModelRc::new(VecModel::from(slint_strings)));
}

fn find_icons_directory() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join("Icons");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }

    if let Ok(app_data) = std::env::var("APPDATA") {
        let candidate = PathBuf::from(app_data).join("ExplorerTags").join("Icons");
        if candidate.is_dir() {
            return candidate;
        }
    }

    PathBuf::from("Icons")
}

fn parse_hex_color(hex: &str) -> Option<[u8; 3]> {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
    let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
    let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
    Some([r, g, b])
}

fn load_ico_as_slint_image(ico_path: &Path) -> Option<slint::Image> {
    if let Ok(img) = slint::Image::load_from_path(ico_path) {
        return Some(img);
    }
    let dyn_img = image::open(ico_path).ok()?;
    let rgba = dyn_img.into_rgba8();
    let (width, height) = rgba.dimensions();

    let buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
        rgba.as_raw(),
        width,
        height,
    );

    Some(slint::Image::from_rgba8(buffer))
}

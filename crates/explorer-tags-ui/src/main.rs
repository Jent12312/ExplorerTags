#![windows_subsystem = "windows"]

slint::include_modules!();

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use slint::{ComponentHandle, Model, ModelRc, SharedPixelBuffer, VecModel};
use explorer_tags_core::{
    apply_folder_settings, get_folder_state, remove_folder_color, remove_folder_tag, reset_folder,
    TagsStore,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = AppWindow::new()?;

    // 1. Получаем пути переданных папок
    let target_folders: Vec<PathBuf> = std::env::args()
        .skip(1)
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect();

    // 2. Загружаем иконки
    let icons_dir = find_icons_directory();
    let loaded_icons = load_all_icons(&icons_dir);
    let loaded_icons_rc = Rc::new(loaded_icons);
    let initial_icon_rc = Rc::new(RefCell::new(None::<slint::Image>));

    // Инициализируем сетку (по умолчанию 0 колонок при старте)
    let mut current_cols = 0;
    recompute_icon_rows(&app, &loaded_icons_rc, 600.0, &mut current_cols);

    // 3. Загружаем теги
    let tags_store = Rc::new(RefCell::new(TagsStore::load()));
    refresh_tags_in_ui(&app, &tags_store.borrow().tags());

    // 4. Инициализируем состояние выделения и шапку
    let target_folders_rc = Rc::new(RefCell::new(target_folders));
    let icons_paths: Vec<String> = loaded_icons_rc.iter().map(|(_, _, p)| p.clone()).collect();
    let icons_paths_rc = Rc::new(icons_paths);
    let custom_icon_path_rc = Rc::new(RefCell::new(None::<PathBuf>));

    update_folder_header(
        &app,
        &target_folders_rc.borrow(),
        &tags_store.borrow().tags(),
        &icons_paths_rc,
        &loaded_icons_rc,
        &initial_icon_rc,
    );

    // 5. Адаптивная авто-подгонка сетки под ширину окна (Fluid Auto-fill)
    let resize_timer = slint::Timer::default();
    let timer_icons = loaded_icons_rc.clone();
    let timer_app = app.as_weak();
    let mut tracked_cols = current_cols;

    resize_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(50), move || {
        if let Some(ui) = timer_app.upgrade() {
            let width = ui.get_visible_grid_width();
            if width > 120.0 {
                recompute_icon_rows(&ui, &timer_icons, width, &mut tracked_cols);
            }
        }
    });

    // 6. Обработчики событий

    // Интерактивное обновление живого предпросмотра при клике на иконку
    let icon_sel_app = app.as_weak();
    let icon_sel_list = loaded_icons_rc.clone();
    let icon_sel_init = initial_icon_rc.clone();
    let custom_path_for_icon = custom_icon_path_rc.clone();
    app.on_icon_selected(move |idx| {
        *custom_path_for_icon.borrow_mut() = None;
        if let Some(ui) = icon_sel_app.upgrade() {
            ui.set_custom_color_active(false);
            if idx >= 0 && (idx as usize) < icon_sel_list.len() {
                ui.set_preview_folder_icon(icon_sel_list[idx as usize].1.clone());
            } else if let Some(init_img) = icon_sel_init.borrow().as_ref() {
                ui.set_preview_folder_icon(init_img.clone());
            }
        }
    });

    // Обработка выбора цвета из палитры
    let color_sel_app = app.as_weak();
    let custom_path_for_color = custom_icon_path_rc.clone();
    app.on_custom_color_selected(move |hex| {
        if let Some(rgb) = parse_hex_color(hex.as_str()) {
            if let Ok(icon_path) = explorer_tags_core::get_or_create_tinted_icon(rgb) {
                if let Some(img) = load_ico_as_slint_image(&icon_path) {
                    if let Some(ui) = color_sel_app.upgrade() {
                        ui.set_preview_folder_icon(img);
                        ui.set_custom_color_active(true);
                        ui.set_selected_icon_index(-1);
                    }
                }
                *custom_path_for_color.borrow_mut() = Some(icon_path);
            }
        }
    });

    // Обработка генерации произвольного цвета по HEX
    let gen_color_app = app.as_weak();
    let custom_path_for_gen = custom_icon_path_rc.clone();
    app.on_generate_color_clicked(move |hex| {
        if let Some(rgb) = parse_hex_color(hex.as_str()) {
            if let Ok(icon_path) = explorer_tags_core::get_or_create_tinted_icon(rgb) {
                if let Some(img) = load_ico_as_slint_image(&icon_path) {
                    if let Some(ui) = gen_color_app.upgrade() {
                        ui.set_preview_folder_icon(img);
                        ui.set_custom_color_active(true);
                        ui.set_selected_icon_index(-1);
                    }
                }
                *custom_path_for_gen.borrow_mut() = Some(icon_path);
            }
        }
    });

    // 1. Системный диалог выбора цвета Windows (полный цветовой круг и спектр)
    let sys_color_app = app.as_weak();
    let custom_path_for_sys = custom_icon_path_rc.clone();
    app.on_open_system_color_picker(move || {
        let init_rgb = [59, 130, 246]; // #3B82F6 по умолчанию
        if let Some(rgb) = show_windows_color_picker(init_rgb) {
            let hex = format!("{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
            if let Ok(icon_path) = explorer_tags_core::get_or_create_tinted_icon(rgb) {
                if let Some(img) = load_ico_as_slint_image(&icon_path) {
                    if let Some(ui) = sys_color_app.upgrade() {
                        ui.set_preview_folder_icon(img);
                        ui.set_custom_color_active(true);
                        ui.set_custom_color_hex(hex.into());
                        ui.set_selected_icon_index(-1);
                    }
                }
                *custom_path_for_sys.borrow_mut() = Some(icon_path);
            }
        }
    });

    // 2. Интерактивные ползунки Оттенок и Яркость (живое обновление без нажатия кнопок)
    let slider_app = app.as_weak();
    let custom_path_for_slider = custom_icon_path_rc.clone();
    app.on_hsl_slider_changed(move |hue, lightness| {
        let rgb = hsl_to_rgb(hue, 0.85, lightness / 100.0);
        let hex = format!("{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
        if let Ok(icon_path) = explorer_tags_core::get_or_create_tinted_icon(rgb) {
            if let Some(img) = load_ico_as_slint_image(&icon_path) {
                if let Some(ui) = slider_app.upgrade() {
                    ui.set_preview_folder_icon(img);
                    ui.set_custom_color_active(true);
                    ui.set_custom_color_hex(hex.into());
                    ui.set_selected_icon_index(-1);
                }
            }
            *custom_path_for_slider.borrow_mut() = Some(icon_path);
        }
    });

    // Выбор папки через диалог, если приложение запущено без аргументов
    let app_weak = app.as_weak();
    let folders_clone = target_folders_rc.clone();
    let tags_store_for_select = tags_store.clone();
    let icons_paths_for_select = icons_paths_rc.clone();
    let icons_for_select = loaded_icons_rc.clone();
    let init_for_select = initial_icon_rc.clone();

    app.on_select_folder_clicked(move || {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            folders_clone.borrow_mut().clear();
            folders_clone.borrow_mut().push(folder);
            if let Some(ui) = app_weak.upgrade() {
                update_folder_header(
                    &ui,
                    &folders_clone.borrow(),
                    &tags_store_for_select.borrow().tags(),
                    &icons_paths_for_select,
                    &icons_for_select,
                    &init_for_select,
                );
            }
        }
    });

    // Применить выбранные цвет и тег
    let app_weak = app.as_weak();
    let folders_clone = target_folders_rc.clone();
    let icons_paths_clone = icons_paths_rc.clone();
    let tags_store_clone = tags_store.clone();
    let custom_path_for_apply = custom_icon_path_rc.clone();

    app.on_apply_clicked(move |icon_idx, tag_idx| {
        let folders = folders_clone.borrow();
        if folders.is_empty() {
            if let Some(ui) = app_weak.upgrade() {
                ui.hide().ok();
            }
            return;
        }

        let custom_borrow = custom_path_for_apply.borrow();
        let icon_path_str: Option<String> = if let Some(p) = custom_borrow.as_ref() {
            Some(p.to_string_lossy().to_string())
        } else if icon_idx >= 0 && (icon_idx as usize) < icons_paths_clone.len() {
            Some(icons_paths_clone[icon_idx as usize].clone())
        } else {
            None
        };
        let icon_path = icon_path_str.as_deref();

        let store = tags_store_clone.borrow();
        let tag = if tag_idx > 0 && (tag_idx as usize - 1) < store.tags().len() {
            Some(store.tags()[tag_idx as usize - 1].as_str())
        } else {
            None
        };

        for f in folders.iter() {
            apply_folder_settings(f, icon_path, tag).ok();
        }

        if let Some(ui) = app_weak.upgrade() {
            ui.hide().ok();
        }
    });

    // Снять только цвет (иконку)
    let app_weak = app.as_weak();
    let folders_clone = target_folders_rc.clone();
    app.on_remove_color_clicked(move || {
        let folders = folders_clone.borrow();
        for f in folders.iter() {
            remove_folder_color(f).ok();
        }
        if let Some(ui) = app_weak.upgrade() {
            ui.hide().ok();
        }
    });

    // Снять только тег
    let app_weak = app.as_weak();
    let folders_clone = target_folders_rc.clone();
    app.on_remove_tag_clicked(move || {
        let folders = folders_clone.borrow();
        for f in folders.iter() {
            remove_folder_tag(f).ok();
        }
        if let Some(ui) = app_weak.upgrade() {
            ui.hide().ok();
        }
    });

    // Полный сброс (и цвет, и тег)
    let app_weak = app.as_weak();
    let folders_clone = target_folders_rc.clone();
    app.on_reset_all_clicked(move || {
        let folders = folders_clone.borrow();
        for f in folders.iter() {
            reset_folder(f).ok();
        }
        if let Some(ui) = app_weak.upgrade() {
            ui.hide().ok();
        }
    });

    // Отмена
    let app_weak = app.as_weak();
    app.on_cancel_clicked(move || {
        if let Some(ui) = app_weak.upgrade() {
            ui.hide().ok();
        }
    });

    // Добавление нового тега
    let app_weak = app.as_weak();
    let tags_store_clone = tags_store.clone();
    app.on_add_tag_confirmed(move |new_tag| {
        let mut store = tags_store_clone.borrow_mut();
        if store.add_tag(new_tag.as_str()) {
            if let Some(ui) = app_weak.upgrade() {
                refresh_tags_in_ui(&ui, store.tags());
                // Выбираем вновь созданный тег
                ui.set_selected_tag_index(store.tags().len() as i32);
            }
        }
    });

    // Удаление тега
    let app_weak = app.as_weak();
    let tags_store_clone = tags_store.clone();
    app.on_delete_tag_clicked(move |tag_idx| {
        let mut store = tags_store_clone.borrow_mut();
        if tag_idx >= 0 && (tag_idx as usize) < store.tags().len() {
            let tag_to_remove = store.tags()[tag_idx as usize].clone();
            if store.remove_tag(&tag_to_remove) {
                if let Some(ui) = app_weak.upgrade() {
                    refresh_tags_in_ui(&ui, store.tags());
                    ui.set_selected_tag_index(0); // Сбрасываем выбор на "Без тега"
                }
            }
        }
    });

    app.run()?;
    Ok(())
}

fn recompute_icon_rows(
    app: &AppWindow,
    loaded_icons: &[(String, slint::Image, String)],
    available_width: f32,
    current_cols: &mut usize,
) {
    let padding_and_scrollbar = 20.0 + 16.0; // 10px padding each side + 16px scrollbar reserve
    let inner_width = (available_width - padding_and_scrollbar).max(120.0);
    let spacing = 8.0;
    let target_slot = 142.0; // target item width + spacing
    let cols = ((inner_width + spacing) / target_slot).floor().clamp(2.0, 16.0) as usize;
    let card_w = ((inner_width - spacing * (cols - 1) as f32) / (cols as f32)).floor();

    app.set_card_width(card_w);

    if cols == *current_cols && app.get_icon_rows().row_count() > 0 {
        return;
    }
    *current_cols = cols;

    let mut rows: Vec<IconRow> = Vec::new();
    let mut current_row = Vec::new();

    for (idx, (title, img, _path)) in loaded_icons.iter().enumerate() {
        current_row.push(IconData {
            id: idx as i32,
            title: title.clone().into(),
            image: img.clone(),
        });

        if current_row.len() == cols {
            rows.push(IconRow {
                items: ModelRc::new(VecModel::from(std::mem::take(&mut current_row))),
            });
        }
    }
    if !current_row.is_empty() {
        rows.push(IconRow {
            items: ModelRc::new(VecModel::from(current_row)),
        });
    }

    app.set_icon_rows(ModelRc::new(VecModel::from(rows)));
}

fn update_folder_header(
    app: &AppWindow,
    folders: &[PathBuf],
    tags: &[String],
    icons_paths: &[String],
    loaded_icons: &[(String, slint::Image, String)],
    initial_icon: &Rc<RefCell<Option<slint::Image>>>,
) {
    // Базовая иконка по умолчанию (первая иконка или стандартная)
    let fallback_icon = loaded_icons.first().map(|(_, img, _)| img.clone()).unwrap_or_default();

    if folders.is_empty() {
        app.set_has_selection(false);
        app.set_folder_name("Папка не выбрана".into());
        app.set_folder_count(0);
        app.set_current_tag("".into());
        app.set_selected_tag_index(0);
        app.set_selected_icon_index(-1);
        app.set_preview_folder_icon(fallback_icon.clone());
        *initial_icon.borrow_mut() = Some(fallback_icon);
    } else {
        app.set_has_selection(true);
        app.set_folder_count(folders.len() as i32);

        if folders.len() == 1 {
            let name = folders[0]
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Папка");
            app.set_folder_name(name.into());

            let state = get_folder_state(&folders[0]);
            let current_tag = state.tag.clone().unwrap_or_default();
            app.set_current_tag(current_tag.clone().into());

            // Автоматически выбираем текущий тег в выпадающем списке
            if !current_tag.is_empty() {
                if let Some(pos) = tags.iter().position(|t| t == &current_tag) {
                    app.set_selected_tag_index((pos + 1) as i32);
                } else {
                    app.set_selected_tag_index(0);
                }
            } else {
                app.set_selected_tag_index(0);
            }

            // Автоматически подсвечиваем текущую иконку в сетке и настраиваем живой предпросмотр
            let mut resolved_icon = fallback_icon.clone();
            let mut matched_idx: i32 = -1;

            if let Some(icon_p) = state.icon_path {
                let norm = icon_p.trim_matches('"');
                let norm_filename = Path::new(norm)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("");

                if let Some(pos) = icons_paths.iter().position(|p| {
                    p.eq_ignore_ascii_case(norm)
                        || (!norm_filename.is_empty()
                            && Path::new(p)
                                .file_name()
                                .and_then(|f| f.to_str())
                                .map(|f| f.eq_ignore_ascii_case(norm_filename))
                                .unwrap_or(false))
                }) {
                    matched_idx = pos as i32;
                    resolved_icon = loaded_icons[pos].1.clone();
                }
            }

            app.set_selected_icon_index(matched_idx);
            app.set_preview_folder_icon(resolved_icon.clone());
            *initial_icon.borrow_mut() = Some(resolved_icon);
        } else {
            app.set_folder_name(format!("Выделено папок: {}", folders.len()).into());
            app.set_current_tag("".into());
            app.set_selected_tag_index(0);
            app.set_selected_icon_index(-1);
            app.set_preview_folder_icon(fallback_icon.clone());
            *initial_icon.borrow_mut() = Some(fallback_icon);
        }
    }
}

fn refresh_tags_in_ui(app: &AppWindow, tags: &[String]) {
    let mut ui_tags = vec!["--- Без тега ---".to_string()];
    ui_tags.extend_from_slice(tags);

    let slint_strings: Vec<slint::SharedString> =
        ui_tags.into_iter().map(slint::SharedString::from).collect();
    app.set_tags(ModelRc::new(VecModel::from(slint_strings)));
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

    let local = PathBuf::from("Icons");
    if local.is_dir() {
        return local;
    }

    PathBuf::from("Icons")
}

fn load_all_icons(icons_dir: &Path) -> Vec<(String, slint::Image, String)> {
    let mut list = Vec::new();
    if !icons_dir.is_dir() {
        return list;
    }

    let Ok(entries) = std::fs::read_dir(icons_dir) else {
        return list;
    };

    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("ico"))
                .unwrap_or(false)
        })
        .collect();

    files.sort();

    for file in files {
        let stem = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Icon")
            .replace(" Folder", "");

        if let Some(img) = load_ico_as_slint_image(&file) {
            let full_path = file.to_string_lossy().to_string();
            list.push((stem, img, full_path));
        }
    }

    list
}

fn load_ico_as_slint_image(ico_path: &Path) -> Option<slint::Image> {
    // 1. Попытка стандартного загрузчика Slint
    if let Ok(img) = slint::Image::load_from_path(ico_path) {
        return Some(img);
    }

    // 2. Декодирование через image crate
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

#[repr(C)]
struct CHOOSECOLORW {
    l_struct_size: u32,
    hwnd_owner: windows::Win32::Foundation::HWND,
    h_instance: windows::Win32::Foundation::HWND,
    rgb_result: u32,
    lp_cust_colors: *mut u32,
    flags: u32,
    l_cust_data: isize,
    lpfn_hook: Option<unsafe extern "system" fn(windows::Win32::Foundation::HWND, u32, usize, isize) -> usize>,
    lp_template_name: *const u16,
}

#[link(name = "comdlg32")]
extern "system" {
    fn ChooseColorW(lpcc: *mut CHOOSECOLORW) -> windows::Win32::Foundation::BOOL;
}

fn show_windows_color_picker(initial_rgb: [u8; 3]) -> Option<[u8; 3]> {
    let mut cust_colors = [0x00FFFFFFu32; 16];
    let init_color = (initial_rgb[0] as u32) | ((initial_rgb[1] as u32) << 8) | ((initial_rgb[2] as u32) << 16);
    let mut cc = CHOOSECOLORW {
        l_struct_size: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwnd_owner: windows::Win32::Foundation::HWND::default(),
        h_instance: windows::Win32::Foundation::HWND::default(),
        rgb_result: init_color,
        lp_cust_colors: cust_colors.as_mut_ptr(),
        flags: 0x01 | 0x02, // CC_RGBINIT | CC_FULLOPEN
        l_cust_data: 0,
        lpfn_hook: None,
        lp_template_name: std::ptr::null(),
    };

    let ok = unsafe { ChooseColorW(&mut cc) };
    if ok.as_bool() {
        let r = (cc.rgb_result & 0xFF) as u8;
        let g = ((cc.rgb_result >> 8) & 0xFF) as u8;
        let b = ((cc.rgb_result >> 16) & 0xFF) as u8;
        Some([r, g, b])
    } else {
        None
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = (h % 360.0) / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if (0.0..1.0).contains(&h_prime) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&h_prime) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&h_prime) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&h_prime) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&h_prime) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = l - c / 2.0;
    [
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

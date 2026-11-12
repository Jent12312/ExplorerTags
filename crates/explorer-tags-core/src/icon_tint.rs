use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

/// RGB to HSL conversion.
/// Returns (h: 0.0..360.0, s: 0.0..1.0, l: 0.0..1.0)
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let l = (max + min) / 2.0;

    if delta.abs() < 1e-5 {
        return (0.0, 0.0, l);
    }

    let s = if l <= 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    let mut h = if (max - rf).abs() < 1e-5 {
        ((gf - bf) / delta) % 6.0
    } else if (max - gf).abs() < 1e-5 {
        ((bf - rf) / delta) + 2.0
    } else {
        ((rf - gf) / delta) + 4.0
    };

    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }

    (h, s.clamp(0.0, 1.0), l.clamp(0.0, 1.0))
}

/// HSL to RGB conversion.
/// h in 0.0..360.0, s in 0.0..1.0, l in 0.0..1.0
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;

    let (r_prime, g_prime, b_prime) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    [
        ((r_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Тинтует цвет пикселя, сохраняя освещенность и бумажный вкладыш.
pub fn tint_pixel(r: u8, g: u8, b: u8, target_h: f32, target_s: f32, target_l: f32) -> [u8; 3] {
    let (_orig_h, orig_s, orig_l) = rgb_to_hsl(r, g, b);

    // Сохраняем белый бумажный лист внутри папки белым
    if orig_l > 0.88 && orig_s < 0.15 {
        return [r, g, b];
    }

    // Сохраняем очень темные линии контура почти черными
    if orig_l < 0.08 {
        return [r, g, b];
    }

    let new_l = if target_l > 0.5 {
        orig_l + (1.0 - orig_l) * ((target_l - 0.5) * 1.2).clamp(0.0, 1.0)
    } else {
        orig_l * (target_l * 2.0).clamp(0.2, 1.0)
    };

    let new_s = (target_s * (0.85 + 0.15 * (1.0 - (orig_l - 0.5).abs()))).clamp(0.0, 1.0);

    hsl_to_rgb(target_h, new_s, new_l.clamp(0.0, 1.0))
}

#[derive(Debug, Clone)]
struct IcoEntryHeader {
    width: u8,
    height: u8,
    color_count: u8,
    reserved: u8,
    planes: u16,
    bit_count: u16,
    bytes_in_res: u32,
    image_offset: u32,
}

/// Применяет HSL перекраску ко всем слоям шаблона ICO и возвращает результирующие байты ICO.
pub fn tint_ico_bytes(template_bytes: &[u8], target_rgb: [u8; 3]) -> Result<Vec<u8>, String> {
    if template_bytes.len() < 6 {
        return Err("Template ICO too small".to_string());
    }

    let reserved = u16::from_le_bytes([template_bytes[0], template_bytes[1]]);
    let ico_type = u16::from_le_bytes([template_bytes[2], template_bytes[3]]);
    let count = u16::from_le_bytes([template_bytes[4], template_bytes[5]]) as usize;

    if reserved != 0 || ico_type != 1 {
        return Err("Invalid ICO header".to_string());
    }

    let dir_size = 6 + count * 16;
    if template_bytes.len() < dir_size {
        return Err("Truncated ICO directory".to_string());
    }

    let (target_h, target_s, target_l) = rgb_to_hsl(target_rgb[0], target_rgb[1], target_rgb[2]);

    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let offset = 6 + i * 16;
        let entry = IcoEntryHeader {
            width: template_bytes[offset],
            height: template_bytes[offset + 1],
            color_count: template_bytes[offset + 2],
            reserved: template_bytes[offset + 3],
            planes: u16::from_le_bytes([template_bytes[offset + 4], template_bytes[offset + 5]]),
            bit_count: u16::from_le_bytes([template_bytes[offset + 6], template_bytes[offset + 7]]),
            bytes_in_res: u32::from_le_bytes([
                template_bytes[offset + 8],
                template_bytes[offset + 9],
                template_bytes[offset + 10],
                template_bytes[offset + 11],
            ]),
            image_offset: u32::from_le_bytes([
                template_bytes[offset + 12],
                template_bytes[offset + 13],
                template_bytes[offset + 14],
                template_bytes[offset + 15],
            ]),
        };
        entries.push(entry);
    }

    // Обрабатываем каждый слой изображения
    let mut modified_images = Vec::with_capacity(count);

    for entry in &entries {
        let start = entry.image_offset as usize;
        let end = start + entry.bytes_in_res as usize;
        if end > template_bytes.len() {
            return Err("ICO image data out of bounds".to_string());
        }

        let img_slice = &template_bytes[start..end];

        if img_slice.starts_with(b"\x89PNG\r\n\x1a\n") {
            // PNG формат (обычно 256x256)
            let dynamic_img = image::load_from_memory(img_slice)
                .map_err(|e| format!("PNG decode error: {}", e))?;
            let mut rgba = dynamic_img.to_rgba8();

            for pixel in rgba.pixels_mut() {
                if pixel[3] > 0 {
                    let tinted = tint_pixel(pixel[0], pixel[1], pixel[2], target_h, target_s, target_l);
                    pixel[0] = tinted[0];
                    pixel[1] = tinted[1];
                    pixel[2] = tinted[2];
                }
            }

            let mut png_buf = Vec::new();
            let mut cursor = Cursor::new(&mut png_buf);
            rgba.write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|e| format!("PNG encode error: {}", e))?;

            modified_images.push(png_buf);
        } else if img_slice.len() >= 40 && u32::from_le_bytes([img_slice[0], img_slice[1], img_slice[2], img_slice[3]]) == 40 {
            // DIB (Windows BITMAPINFOHEADER)
            let mut dib_bytes = img_slice.to_vec();
            let width = i32::from_le_bytes([dib_bytes[4], dib_bytes[5], dib_bytes[6], dib_bytes[7]]).abs() as usize;
            let height_doubled = i32::from_le_bytes([dib_bytes[8], dib_bytes[9], dib_bytes[10], dib_bytes[11]]).abs() as usize;
            let height = height_doubled / 2;
            let bit_count = u16::from_le_bytes([dib_bytes[14], dib_bytes[15]]) as usize;

            if bit_count == 32 {
                let pixel_data_offset = 40;
                let pixel_bytes_count = width * height * 4;

                if dib_bytes.len() >= pixel_data_offset + pixel_bytes_count {
                    for chunk in dib_bytes[pixel_data_offset..pixel_data_offset + pixel_bytes_count].chunks_exact_mut(4) {
                        let b = chunk[0];
                        let g = chunk[1];
                        let r = chunk[2];
                        let a = chunk[3];

                        if a > 0 {
                            let tinted = tint_pixel(r, g, b, target_h, target_s, target_l);
                            chunk[0] = tinted[2]; // B
                            chunk[1] = tinted[1]; // G
                            chunk[2] = tinted[0]; // R
                        }
                    }
                }
            }

            modified_images.push(dib_bytes);
        } else {
            // Неизвестный формат - сохраняем как есть
            modified_images.push(img_slice.to_vec());
        }
    }

    // Собираем обратно ICO файл
    let mut out = Vec::new();
    out.extend_from_slice(&template_bytes[0..6]); // Reserved, Type, Count

    let mut current_offset = (6 + count * 16) as u32;

    for (i, entry) in entries.iter().enumerate() {
        let img_len = modified_images[i].len() as u32;
        out.push(entry.width);
        out.push(entry.height);
        out.push(entry.color_count);
        out.push(entry.reserved);
        out.extend_from_slice(&entry.planes.to_le_bytes());
        out.extend_from_slice(&entry.bit_count.to_le_bytes());
        out.extend_from_slice(&img_len.to_le_bytes());
        out.extend_from_slice(&current_offset.to_le_bytes());

        current_offset += img_len;
    }

    for img_data in modified_images {
        out.extend_from_slice(&img_data);
    }

    Ok(out)
}

/// Находит эталонный серый значок Windows 11
pub fn find_template_icon() -> Result<PathBuf, String> {
    // 1. Проверяем AppData
    if let Ok(app_data) = std::env::var("APPDATA") {
        let p = PathBuf::from(app_data).join("ExplorerTags").join("Icons").join("Gray 1 Folder.ico");
        if p.is_file() {
            return Ok(p);
        }
    }

    // 2. Проверяем каталог приложения
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(parent) = exe_dir.parent() {
            let p = parent.join("Icons").join("Gray 1 Folder.ico");
            if p.is_file() {
                return Ok(p);
            }
        }
    }

    // 3. Проверяем относительный путь
    let relative = PathBuf::from("Icons").join("Gray 1 Folder.ico");
    if relative.is_file() {
        return Ok(relative);
    }

    Err("Шаблон Gray 1 Folder.ico не найден".to_string())
}

/// Получает или генерирует кастомную иконку папки целевого цвета (RGB).
/// Кэширует в %APPDATA%\ExplorerTags\CustomIcons\folder_<HEX>.ico
pub fn get_or_create_tinted_icon(target_rgb: [u8; 3]) -> Result<PathBuf, String> {
    let hex = format!("{:02X}{:02X}{:02X}", target_rgb[0], target_rgb[1], target_rgb[2]);

    let app_dir = if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("ExplorerTags")
    } else {
        PathBuf::from(".").join("ExplorerTags")
    };

    let cache_dir = app_dir.join("CustomIcons");
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;

    let icon_path = cache_dir.join(format!("folder_{}.ico", hex));
    if icon_path.is_file() {
        return Ok(icon_path);
    }

    let template_path = find_template_icon()?;
    let template_bytes = fs::read(&template_path).map_err(|e| e.to_string())?;

    let tinted_bytes = tint_ico_bytes(&template_bytes, target_rgb)?;
    fs::write(&icon_path, tinted_bytes).map_err(|e| e.to_string())?;

    Ok(icon_path)
}

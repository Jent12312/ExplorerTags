pub mod keyboard_hook;
pub mod selection;

use std::fs;
use std::path::Path;
use slint::SharedPixelBuffer;

pub struct FilePreviewData {
    pub file_name: String,
    pub file_size: String,
    pub file_type: String,
    pub is_image: bool,
    pub is_text: bool,
    pub image: Option<slint::Image>,
    pub text: String,
}

pub fn load_file_preview(path: &Path) -> FilePreviewData {
    let file_name = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("Файл")
        .to_string();

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let metadata = fs::metadata(path).ok();
    let file_size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
    let file_size = format_file_size(file_size_bytes);

    let is_img = matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "ico" | "gif"
    );

    let is_txt = matches!(
        ext.as_str(),
        "txt"
            | "rs"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "py"
            | "js"
            | "ts"
            | "json"
            | "toml"
            | "yaml"
            | "yml"
            | "xml"
            | "html"
            | "css"
            | "md"
            | "log"
            | "ini"
            | "bat"
            | "cmd"
            | "ps1"
            | "iss"
    );

    let mut preview_img = None;
    let mut text_content = String::new();

    if is_img {
        if let Ok(dyn_img) = image::open(path) {
            let rgba = dyn_img.into_rgba8();
            let (w, h) = rgba.dimensions();
            let buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                rgba.as_raw(),
                w,
                h,
            );
            preview_img = Some(slint::Image::from_rgba8(buffer));
        }
    } else if is_txt {
        if let Ok(bytes) = fs::read(path) {
            let slice = if bytes.len() > 64 * 1024 {
                &bytes[..64 * 1024]
            } else {
                &bytes[..]
            };
            text_content = String::from_utf8_lossy(slice).to_string();
            if bytes.len() > 64 * 1024 {
                text_content.push_str("\n\n... [Файл обрезан для быстрого предпросмотра]");
            }
        }
    }

    FilePreviewData {
        file_name,
        file_size,
        file_type: ext.to_uppercase(),
        is_image: is_img && preview_img.is_some(),
        is_text: is_txt,
        image: preview_img,
        text: text_content,
    }
}

pub fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} Б", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} КБ", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} МБ", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} ГБ", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

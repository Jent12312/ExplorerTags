use explorer_tags_core::icon_tint::{tint_ico_bytes, rgb_to_hsl, hsl_to_rgb};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_rgb_hsl_roundtrip() {
    let red = [255, 0, 0];
    let (h, s, l) = rgb_to_hsl(red[0], red[1], red[2]);
    assert!((h - 0.0).abs() < 1e-4);
    assert!((s - 1.0).abs() < 1e-4);
    assert!((l - 0.5).abs() < 1e-4);

    let [r2, g2, b2] = hsl_to_rgb(h, s, l);
    assert_eq!([r2, g2, b2], red);
}

#[test]
fn test_tint_ico_bytes_valid() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let icon_path = manifest_dir.join("../../Icons/Gray 1 Folder.ico");
    assert!(icon_path.is_file(), "Template icon must exist at {:?}", icon_path);

    let template_bytes = fs::read(&icon_path).expect("Read template icon");
    let target_color = [255, 60, 0]; // Bright orange-red

    let tinted_bytes = tint_ico_bytes(&template_bytes, target_color).expect("Tint icon failed");

    // Check ICO header: 0x0000 (reserved), 0x0001 (ico type)
    assert!(tinted_bytes.len() > 6);
    assert_eq!(&tinted_bytes[0..4], &[0, 0, 1, 0]);

    // Check count of images in ICO
    let count = u16::from_le_bytes([tinted_bytes[4], tinted_bytes[5]]);
    assert_eq!(count, 8, "Expected 8 icon sizes from the Windows 11 template");

    // Try decoding with image crate
    let img = image::load_from_memory_with_format(&tinted_bytes, image::ImageFormat::Ico)
        .expect("Tinted icon must be decodable by image crate");
    assert!(img.width() > 0 && img.height() > 0);
}

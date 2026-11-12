fn main() {
    slint_build::compile("ui/app.slint").unwrap();
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../Icons/Rust 1 Folder.ico");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile icon resource: {}", e);
        }
    }
}

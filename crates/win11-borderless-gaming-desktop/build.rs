#[cfg(target_os = "windows")]
fn main() {
    let icon = "assets/app.ico";

    println!("cargo:rerun-if-changed={icon}");

    let mut res = winres::WindowsResource::new();
    res.set_icon(icon);

    res.set("CompanyName", "Sylvain Benner");
    res.set("FileDescription", "Borderless Gaming Desktop Toggle");
    res.set("ProductName", "win11-borderless-gaming-desktop");
    res.set("OriginalFilename", "win11-borderless-gaming-desktop.exe");
    res.set("LegalCopyright", "© Sylvain Benner");
    res.set("FileVersion", env!("CARGO_PKG_VERSION"));
    res.set("ProductVersion", env!("CARGO_PKG_VERSION"));

    res.compile()
        .expect("Windows resource compilation should succeed");
}

#[cfg(not(target_os = "windows"))]
fn main() {}

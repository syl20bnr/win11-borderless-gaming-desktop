#[cfg(target_os = "windows")]
fn main() {
    use std::path::Path;

    let icon_path = "assets/taskbar-autohide.ico";

    if Path::new(icon_path).exists() {
        let mut res = winres::WindowsResource::new();
        res.set_icon(icon_path);
        res.compile().expect("failed to compile Windows resources");
    } else {
        println!(
            "cargo:warning=Windows icon not found at {icon_path}. Add an .ico file there to embed an executable icon."
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {}

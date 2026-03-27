#[cfg(target_os = "windows")]
mod app {
    use std::mem::size_of;
    use windows::Win32::UI::Shell::{
        APPBARDATA, ABS_AUTOHIDE, ABM_GETSTATE, ABM_SETSTATE, SHAppBarMessage,
    };

    pub fn run() {
        unsafe {
            let mut appbar = APPBARDATA {
                cbSize: size_of::<APPBARDATA>() as u32,
                ..Default::default()
            };

            let current_state = SHAppBarMessage(ABM_GETSTATE, Some(&mut appbar));
            let toggled_state = if current_state & ABS_AUTOHIDE != 0 {
                current_state & !ABS_AUTOHIDE
            } else {
                current_state | ABS_AUTOHIDE
            };

            appbar.lParam = toggled_state as isize;
            let _ = SHAppBarMessage(ABM_SETSTATE, Some(&mut appbar));
        }
    }
}

#[cfg(target_os = "windows")]
fn main() {
    app::run();
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("This utility only runs on Windows.");
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

#[cfg(feature = "gui")]
mod behavior;
#[cfg(feature = "gui")]
mod display;
#[cfg(feature = "gui")]
mod gui;
#[cfg(feature = "sound")]
mod sound;

#[cfg(feature = "gui")]
fn show_gui_startup_error(error: &eframe::Error) {
    use windows::{
        Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW},
        core::{HSTRING, w},
    };

    let message = HSTRING::from(format!(
        "Borderless Gaming Desktop could not start.\n\n{error}"
    ));
    let _ = unsafe {
        MessageBoxW(
            None,
            &message,
            w!("Borderless Gaming Desktop"),
            MB_OK | MB_ICONERROR,
        )
    };
}

fn main() -> std::process::ExitCode {
    #[cfg(feature = "gui")]
    {
        if let Err(error) = gui::run() {
            eprintln!("Could not start the GUI: {error}");
            show_gui_startup_error(&error);
            return std::process::ExitCode::FAILURE;
        }
    }

    #[cfg(not(feature = "gui"))]
    app::run();

    std::process::ExitCode::SUCCESS
}

use std::{ffi::OsStr, os::windows::ffi::OsStrExt as _, path::Path};

use windows::{
    Win32::{
        Foundation::{COLORREF, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HWND},
        System::Registry::{
            HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW,
            RegSetKeyValueW,
        },
        UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongPtrW, LWA_ALPHA, SetLayeredWindowAttributes,
            SetWindowLongPtrW, WS_EX_LAYERED,
        },
    },
    core::w,
};

const RUN_KEY: windows::core::PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const RUN_VALUE_NAME: windows::core::PCWSTR = w!("Borderless Gaming Desktop");
pub const START_MINIMIZED_ARGUMENT: &str = "--minimized";
pub const MAX_TRANSPARENCY_PERCENT: u8 = 80;

/// Returns whether the application has a per-user Windows login entry.
pub fn startup_at_login_enabled() -> Result<bool, String> {
    let mut byte_count = 0;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            RUN_VALUE_NAME,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut byte_count),
        )
    };

    match status {
        ERROR_SUCCESS => Ok(true),
        ERROR_FILE_NOT_FOUND => Ok(false),
        error => Err(format!(
            "Could not read the Windows login setting: {}",
            error.to_hresult().message()
        )),
    }
}

/// Creates, updates, or removes the current user's Windows login entry.
pub fn set_startup_at_login(enabled: bool, minimized: bool) -> Result<(), String> {
    if !enabled {
        let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE_NAME) };
        return match status {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
            error => Err(format!(
                "Could not remove the Windows login setting: {}",
                error.to_hresult().message()
            )),
        };
    }

    let executable = std::env::current_exe()
        .map_err(|error| format!("Could not locate the application executable: {error}"))?;
    let command = startup_command(&executable, minimized);
    let byte_count =
        u32::try_from(command.len().saturating_mul(size_of::<u16>())).map_err(|_| {
            "The application path is too long for the Windows login setting.".to_owned()
        })?;
    let status = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            RUN_VALUE_NAME,
            REG_SZ.0,
            Some(command.as_ptr().cast()),
            byte_count,
        )
    };

    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!(
            "Could not update the Windows login setting: {}",
            status.to_hresult().message()
        ))
    }
}

/// Applies whole-window opacity while keeping the window safely interactive.
pub fn set_window_transparency(hwnd: HWND, transparency_percent: u8) -> Result<(), String> {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
    }

    unsafe {
        SetLayeredWindowAttributes(
            hwnd,
            COLORREF(0),
            transparency_alpha(transparency_percent),
            LWA_ALPHA,
        )
    }
    .map_err(|error| format!("Could not change window transparency: {error}"))
}

fn startup_command(executable: &Path, minimized: bool) -> Vec<u16> {
    let mut command = vec!['"' as u16];
    command.extend(executable.as_os_str().encode_wide());
    command.push('"' as u16);
    if minimized {
        command.push(' ' as u16);
        command.extend(OsStr::new(START_MINIMIZED_ARGUMENT).encode_wide());
    }
    command.push(0);
    command
}

fn transparency_alpha(transparency_percent: u8) -> u8 {
    let transparency = transparency_percent.min(MAX_TRANSPARENCY_PERCENT) as f32 / 100.0;
    ((1.0 - transparency) * u8::MAX as f32).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_command_quotes_paths_and_adds_the_minimized_argument() {
        let command = startup_command(
            Path::new(r"C:\Program Files\Borderless Gaming\app.exe"),
            true,
        );
        let command = String::from_utf16(&command[..command.len() - 1]).unwrap();

        assert_eq!(
            command,
            r#""C:\Program Files\Borderless Gaming\app.exe" --minimized"#
        );
    }

    #[test]
    fn login_command_can_start_normally() {
        let command = startup_command(Path::new(r"C:\Games\app.exe"), false);
        let command = String::from_utf16(&command[..command.len() - 1]).unwrap();

        assert_eq!(command, r#""C:\Games\app.exe""#);
    }

    #[test]
    fn transparency_is_capped_before_the_window_becomes_hard_to_recover() {
        assert_eq!(transparency_alpha(0), 255);
        assert_eq!(transparency_alpha(50), 128);
        assert_eq!(transparency_alpha(80), 51);
        assert_eq!(transparency_alpha(100), 51);
    }
}

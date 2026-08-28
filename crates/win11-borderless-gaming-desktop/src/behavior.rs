use std::{ffi::OsStr, os::windows::ffi::OsStrExt as _, path::Path};

use windows::{
    Win32::{
        Foundation::{COLORREF, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HWND},
        Globalization::{CSTR_EQUAL, CompareStringOrdinal},
        System::Registry::{
            HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW,
            RegSetKeyValueW,
        },
        UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongPtrW, IsWindowVisible, LWA_ALPHA, SetLayeredWindowAttributes,
            SetWindowLongPtrW, WS_EX_LAYERED,
        },
    },
    core::w,
};

const RUN_KEY: windows::core::PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const RUN_VALUE_NAME: windows::core::PCWSTR = w!("Borderless Gaming Desktop");
pub const START_MINIMIZED_ARGUMENT: &str = "--minimized";
pub const MAX_TRANSPARENCY_PERCENT: u8 = 80;

/// Returns whether the current executable has a per-user Windows login entry.
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
        ERROR_FILE_NOT_FOUND => return Ok(false),
        ERROR_SUCCESS => {}
        error => {
            return Err(format!(
                "Could not read the Windows login setting: {}",
                error.to_hresult().message()
            ));
        }
    }

    let mut command = vec![0_u16; (byte_count as usize).div_ceil(size_of::<u16>()).max(1)];
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            RUN_VALUE_NAME,
            RRF_RT_REG_SZ,
            None,
            Some(command.as_mut_ptr().cast()),
            Some(&mut byte_count),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!(
            "Could not read the Windows login setting: {}",
            status.to_hresult().message()
        ));
    }

    let executable = std::env::current_exe()
        .map_err(|error| format!("Could not locate the application executable: {error}"))?;
    Ok(startup_command_targets_executable(&command, &executable))
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

/// Returns whether Windows has made the native app window visible.
pub fn window_is_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd) }.as_bool()
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

fn startup_command_targets_executable(command: &[u16], executable: &Path) -> bool {
    let Some(registered_executable) = startup_command_executable(command) else {
        return false;
    };
    let executable = executable.as_os_str().encode_wide().collect::<Vec<_>>();

    unsafe { CompareStringOrdinal(registered_executable, &executable, true) == CSTR_EQUAL }
}

fn startup_command_executable(command: &[u16]) -> Option<&[u16]> {
    let command = command.split(|unit| *unit == 0).next().unwrap_or_default();
    let executable = if command.first() == Some(&('"' as u16)) {
        let closing_quote = command[1..].iter().position(|unit| *unit == '"' as u16)? + 1;
        if command
            .get(closing_quote + 1)
            .is_some_and(|unit| *unit != b' ' as u16 && *unit != b'\t' as u16)
        {
            return None;
        }
        &command[1..closing_quote]
    } else {
        let end = command
            .iter()
            .position(|unit| *unit == b' ' as u16 || *unit == b'\t' as u16)
            .unwrap_or(command.len());
        &command[..end]
    };

    (!executable.is_empty()).then_some(executable)
}

fn transparency_alpha(transparency_percent: u8) -> u8 {
    let transparency = transparency_percent.min(MAX_TRANSPARENCY_PERCENT) as f32 / 100.0;
    ((1.0 - transparency) * u8::MAX as f32).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetLayeredWindowAttributes,
        LAYERED_WINDOW_ATTRIBUTES_FLAGS, SW_SHOWNOACTIVATE, ShowWindow, WINDOW_EX_STYLE,
        WS_OVERLAPPED,
    };

    struct HiddenTestWindow(HWND);

    impl Drop for HiddenTestWindow {
        fn drop(&mut self) {
            let _ = unsafe { DestroyWindow(self.0) };
        }
    }

    fn hidden_test_window() -> HiddenTestWindow {
        HiddenTestWindow(
            unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("Transparency startup test"),
                    WS_OVERLAPPED,
                    -32_000,
                    -32_000,
                    16,
                    16,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .expect("the built-in STATIC window class should be available"),
        )
    }

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
    fn login_command_for_the_current_path_is_enabled_regardless_of_arguments() {
        let executable = Path::new(r"C:\Games\Borderless Gaming Desktop.exe");

        assert!(startup_command_targets_executable(
            &startup_command(executable, false),
            executable
        ));
        assert!(startup_command_targets_executable(
            &startup_command(executable, true),
            executable
        ));
    }

    #[test]
    fn login_command_path_comparison_ignores_windows_path_casing() {
        let command = startup_command(Path::new(r"C:\GAMES\ÉCRAN.EXE"), false);

        assert!(startup_command_targets_executable(
            &command,
            Path::new(r"c:\games\écran.exe")
        ));
    }

    #[test]
    fn login_command_for_a_previous_path_is_not_enabled() {
        let command = startup_command(Path::new(r"C:\Old\app.exe"), true);

        assert!(!startup_command_targets_executable(
            &command,
            Path::new(r"C:\New\app.exe")
        ));
    }

    #[test]
    fn malformed_login_commands_are_not_enabled() {
        assert!(!startup_command_targets_executable(
            &[0],
            Path::new(r"C:\Games\app.exe")
        ));
        assert!(!startup_command_targets_executable(
            &r#""C:\Games\app.exe"#.encode_utf16().chain([0]).collect::<Vec<_>>(),
            Path::new(r"C:\Games\app.exe")
        ));
        assert!(!startup_command_targets_executable(
            &r#""C:\Games\app.exe".old --minimized"#.encode_utf16().chain([0]).collect::<Vec<_>>(),
            Path::new(r"C:\Games\app.exe")
        ));
    }

    #[test]
    fn transparency_is_capped_before_the_window_becomes_hard_to_recover() {
        assert_eq!(transparency_alpha(0), 255);
        assert_eq!(transparency_alpha(50), 128);
        assert_eq!(transparency_alpha(80), 51);
        assert_eq!(transparency_alpha(100), 51);
    }

    #[test]
    fn transparency_is_installed_while_a_native_window_is_still_hidden() {
        let window = hidden_test_window();

        assert!(!window_is_visible(window.0));
        set_window_transparency(window.0, 40).unwrap();

        let style = unsafe { GetWindowLongPtrW(window.0, GWL_EXSTYLE) };
        assert_ne!(style & WS_EX_LAYERED.0 as isize, 0);
        let mut alpha = 0;
        let mut flags = LAYERED_WINDOW_ATTRIBUTES_FLAGS(0);
        unsafe { GetLayeredWindowAttributes(window.0, None, Some(&mut alpha), Some(&mut flags)) }
            .unwrap();
        assert_eq!(alpha, transparency_alpha(40));
        assert_eq!(flags, LWA_ALPHA);
    }

    #[test]
    fn transparency_can_be_repaired_after_show_rewrites_the_window_style() {
        let window = hidden_test_window();
        set_window_transparency(window.0, 40).unwrap();
        let _ = unsafe { ShowWindow(window.0, SW_SHOWNOACTIVATE) };
        assert!(window_is_visible(window.0));

        // Model winit's Visible(true) style diff, which replaces GWL_EXSTYLE
        // without preserving the WS_EX_LAYERED bit added by the app.
        let style = unsafe { GetWindowLongPtrW(window.0, GWL_EXSTYLE) };
        unsafe {
            SetWindowLongPtrW(window.0, GWL_EXSTYLE, style & !(WS_EX_LAYERED.0 as isize));
        }
        assert_eq!(
            unsafe { GetWindowLongPtrW(window.0, GWL_EXSTYLE) } & WS_EX_LAYERED.0 as isize,
            0
        );

        set_window_transparency(window.0, 40).unwrap();

        let mut alpha = 0;
        let mut flags = LAYERED_WINDOW_ATTRIBUTES_FLAGS(0);
        unsafe { GetLayeredWindowAttributes(window.0, None, Some(&mut alpha), Some(&mut flags)) }
            .unwrap();
        assert_eq!(alpha, transparency_alpha(40));
        assert_eq!(flags, LWA_ALPHA);
    }
}

use std::mem::size_of;

use windows::{
    Win32::{
        Foundation::{COLORREF, E_FAIL, HWND, LPARAM, RPC_E_CHANGED_MODE},
        System::Com::{
            CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
        },
        UI::{
            Shell::{
                ABM_GETSTATE, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA, DesktopWallpaper,
                IDesktopWallpaper, IShellDispatch, SHAppBarMessage, Shell,
            },
            WindowsAndMessaging::{FindWindowExW, FindWindowW, SW_HIDE, SW_SHOW, ShowWindow},
        },
    },
    core::{Error, Result, w},
};

/// Runtime choices for the behaviors applied by a mode transition.
#[derive(Clone, Copy, Debug, Default)]
pub struct ToggleOptions {
    pub taskbar_auto_hide: bool,
    pub desktop_icons: bool,
    pub desktop_background: bool,
    pub minimize_all_windows: bool,
}

/// The result of applying the selected actions for one mode transition.
#[derive(Debug, Default)]
pub struct ActionReport {
    pub errors: Vec<String>,
}

struct ComApartment {
    uninitialize: bool,
}

impl ComApartment {
    fn initialize() -> Result<Self> {
        match unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok() {
            Ok(()) => Ok(Self { uninitialize: true }),
            // A GUI dependency may already have initialized this thread as an MTA.
            // COM is still available; only the apartment model cannot be changed.
            Err(error) if error.code() == RPC_E_CHANGED_MODE => Ok(Self {
                uninitialize: false,
            }),
            Err(error) => Err(error),
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.uninitialize {
            unsafe { CoUninitialize() };
        }
    }
}

fn find_desktop_def_view() -> Result<HWND> {
    unsafe {
        if let Ok(progman) = FindWindowW(w!("Progman"), None)
            && let Ok(def_view) = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None)
        {
            return Ok(def_view);
        }

        let mut worker = HWND(std::ptr::null_mut());

        while let Ok(next_worker) = FindWindowExW(None, Some(worker), w!("WorkerW"), None) {
            worker = next_worker;

            if let Ok(def_view) = FindWindowExW(Some(worker), None, w!("SHELLDLL_DefView"), None) {
                return Ok(def_view);
            }
        }
    }

    Err(Error::new(
        E_FAIL,
        "Windows could not find the desktop icon view",
    ))
}

fn set_desktop_icons_visible(visible: bool) -> Result<()> {
    let def_view = find_desktop_def_view()?;
    let list_view = unsafe { FindWindowExW(Some(def_view), None, w!("SysListView32"), None) }?;

    // ShowWindow returns the previous visibility, not an error indicator.
    let _ = unsafe { ShowWindow(list_view, if visible { SW_SHOW } else { SW_HIDE }) };
    Ok(())
}

fn apply_solid_background(enable_black_background: bool) -> Result<()> {
    let _apartment = ComApartment::initialize()?;
    let wallpaper: IDesktopWallpaper =
        unsafe { CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL) }?;

    if enable_black_background {
        unsafe {
            wallpaper.SetBackgroundColor(COLORREF(0x000000))?;
            wallpaper.Enable(false)?;
        }
    } else {
        unsafe { wallpaper.Enable(true) }?;
    }

    Ok(())
}

fn minimize_all_windows() -> Result<()> {
    let _apartment = ComApartment::initialize()?;
    let shell: IShellDispatch = unsafe { CoCreateInstance(&Shell, None, CLSCTX_ALL) }?;
    unsafe { shell.MinimizeAll() }
}

/// Returns whether Windows' primary taskbar currently has auto-hide enabled.
pub fn taskbar_auto_hide_enabled() -> bool {
    let mut appbar = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        ..Default::default()
    };

    let current_state = unsafe { SHAppBarMessage(ABM_GETSTATE, &mut appbar) };
    (current_state & ABS_AUTOHIDE as usize) != 0
}

fn set_taskbar_auto_hide_enabled(enabled: bool) -> bool {
    let mut appbar = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        ..Default::default()
    };

    let current_state = unsafe { SHAppBarMessage(ABM_GETSTATE, &mut appbar) };
    let autohide_flag = ABS_AUTOHIDE as usize;
    let next_state = if enabled {
        current_state | autohide_flag
    } else {
        current_state & !autohide_flag
    };

    appbar.lParam = LPARAM(next_state as isize);
    let _ = unsafe { SHAppBarMessage(ABM_SETSTATE, &mut appbar) };
    taskbar_auto_hide_enabled()
}

/// Applies every selected Gaming Mode action, collecting independent failures.
pub fn activate(options: ToggleOptions) -> ActionReport {
    let mut errors = Vec::new();

    if options.taskbar_auto_hide && !set_taskbar_auto_hide_enabled(true) {
        errors.push("Taskbar auto-hide: Windows did not enable the requested state".to_owned());
    }

    if options.desktop_icons
        && let Err(error) = set_desktop_icons_visible(false)
    {
        errors.push(format!("Desktop icons: {error}"));
    }

    if options.desktop_background
        && let Err(error) = apply_solid_background(true)
    {
        errors.push(format!("Desktop background: {error}"));
    }

    if options.minimize_all_windows
        && let Err(error) = minimize_all_windows()
    {
        errors.push(format!("Minimize windows: {error}"));
    }

    ActionReport { errors }
}

/// Restores every selected reversible action and the exact captured taskbar state.
///
/// Taskbar restoration intentionally runs last. Any reported error should keep the
/// GUI-owned mode active so the same restoration can be retried.
pub fn restore(options: ToggleOptions, original_taskbar_auto_hide: Option<bool>) -> ActionReport {
    let mut errors = Vec::new();

    if options.desktop_icons
        && let Err(error) = set_desktop_icons_visible(true)
    {
        errors.push(format!("Desktop icons: {error}"));
    }

    if options.desktop_background
        && let Err(error) = apply_solid_background(false)
    {
        errors.push(format!("Desktop background: {error}"));
    }

    if options.taskbar_auto_hide {
        match original_taskbar_auto_hide {
            Some(original) if set_taskbar_auto_hide_enabled(original) != original => errors
                .push("Taskbar auto-hide: Windows did not restore the original state".to_owned()),
            Some(_) => {}
            None => errors.push(
                "Taskbar auto-hide: The original state is unavailable and could not be restored"
                    .to_owned(),
            ),
        }
    }

    ActionReport { errors }
}

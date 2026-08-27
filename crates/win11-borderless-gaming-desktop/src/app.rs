use std::mem::size_of;

use windows::Win32::{
    Foundation::LPARAM,
    UI::Shell::{ABM_GETSTATE, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage},
};

#[cfg(feature = "desktop-icons")]
use windows::core::{Error, w};

#[cfg(any(
    feature = "desktop-icons",
    feature = "desktop-background",
    feature = "minimize-all-windows"
))]
use windows::core::Result;

#[cfg(feature = "desktop-icons")]
use windows::Win32::{
    Foundation::{E_FAIL, HWND},
    UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, SW_HIDE, SW_SHOW, ShowWindow},
};

#[cfg(any(feature = "desktop-background", feature = "minimize-all-windows"))]
use windows::Win32::{
    Foundation::RPC_E_CHANGED_MODE,
    System::Com::{
        CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
    },
};

#[cfg(feature = "desktop-background")]
use windows::Win32::{
    Foundation::COLORREF,
    UI::Shell::{DesktopWallpaper, IDesktopWallpaper},
};

#[cfg(feature = "minimize-all-windows")]
use windows::Win32::UI::Shell::{IShellDispatch, Shell};

/// Runtime choices for the optional behaviors compiled into the executable.
#[derive(Clone, Copy, Debug, Default)]
pub struct ToggleOptions {
    #[cfg(feature = "desktop-icons")]
    pub desktop_icons: bool,
    #[cfg(feature = "desktop-background")]
    pub desktop_background: bool,
    #[cfg(feature = "minimize-all-windows")]
    pub minimize_all_windows: bool,
}

#[cfg(not(feature = "gui"))]
impl ToggleOptions {
    /// Enables every behavior that is present in this build.
    pub const fn all() -> Self {
        Self {
            #[cfg(feature = "desktop-icons")]
            desktop_icons: true,
            #[cfg(feature = "desktop-background")]
            desktop_background: true,
            #[cfg(feature = "minimize-all-windows")]
            minimize_all_windows: true,
        }
    }
}

/// The result of applying the main mode toggle.
#[cfg(feature = "gui")]
#[derive(Debug)]
pub struct ToggleReport {
    pub gaming_mode_enabled: bool,
    pub errors: Vec<String>,
    pub optional_actions_failed: bool,
}

#[cfg(any(feature = "desktop-background", feature = "minimize-all-windows"))]
struct ComApartment {
    uninitialize: bool,
}

#[cfg(any(feature = "desktop-background", feature = "minimize-all-windows"))]
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

#[cfg(any(feature = "desktop-background", feature = "minimize-all-windows"))]
impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.uninitialize {
            unsafe { CoUninitialize() };
        }
    }
}

#[cfg(feature = "desktop-icons")]
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

#[cfg(feature = "desktop-icons")]
fn set_desktop_icons_visible(visible: bool) -> Result<()> {
    let def_view = find_desktop_def_view()?;
    let list_view = unsafe { FindWindowExW(Some(def_view), None, w!("SysListView32"), None) }?;

    // ShowWindow returns the previous visibility, not an error indicator.
    let _ = unsafe { ShowWindow(list_view, if visible { SW_SHOW } else { SW_HIDE }) };
    Ok(())
}

#[cfg(feature = "desktop-background")]
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

#[cfg(feature = "minimize-all-windows")]
fn minimize_all_windows() -> Result<()> {
    let _apartment = ComApartment::initialize()?;
    let shell: IShellDispatch = unsafe { CoCreateInstance(&Shell, None, CLSCTX_ALL) }?;
    unsafe { shell.MinimizeAll() }
}

/// Returns whether Windows' primary taskbar currently has auto-hide enabled.
pub fn gaming_mode_enabled() -> bool {
    let mut appbar = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        ..Default::default()
    };

    let current_state = unsafe { SHAppBarMessage(ABM_GETSTATE, &mut appbar) };
    (current_state & ABS_AUTOHIDE as usize) != 0
}

fn set_gaming_mode_enabled(enabled: bool) -> bool {
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
    gaming_mode_enabled()
}

fn apply_optional_actions(options: ToggleOptions, enable_gaming_mode: bool) -> Vec<String> {
    #[cfg(any(
        feature = "desktop-icons",
        feature = "desktop-background",
        feature = "minimize-all-windows"
    ))]
    let mut errors = Vec::new();
    #[cfg(not(any(
        feature = "desktop-icons",
        feature = "desktop-background",
        feature = "minimize-all-windows"
    )))]
    let errors = Vec::new();

    #[cfg(not(any(
        feature = "desktop-icons",
        feature = "desktop-background",
        feature = "minimize-all-windows"
    )))]
    let _ = options;
    #[cfg(not(any(
        feature = "desktop-icons",
        feature = "desktop-background",
        feature = "minimize-all-windows"
    )))]
    let _ = enable_gaming_mode;

    #[cfg(feature = "desktop-icons")]
    if options.desktop_icons
        && let Err(error) = set_desktop_icons_visible(!enable_gaming_mode)
    {
        errors.push(format!("Desktop icons: {error}"));
    }

    #[cfg(feature = "desktop-background")]
    if options.desktop_background
        && let Err(error) = apply_solid_background(enable_gaming_mode)
    {
        errors.push(format!("Desktop background: {error}"));
    }

    #[cfg(feature = "minimize-all-windows")]
    if enable_gaming_mode
        && options.minimize_all_windows
        && let Err(error) = minimize_all_windows()
    {
        errors.push(format!("Minimize windows: {error}"));
    }

    errors
}

/// Toggles the main gaming state and applies the selected compiled-in behaviors.
fn apply_toggle(options: ToggleOptions) -> (bool, Vec<String>, bool) {
    let enable_gaming_mode = !gaming_mode_enabled();

    // Restore selected desktop behaviors before leaving gaming mode. If any
    // restore fails, keep the main mode active so the same button can retry it.
    if !enable_gaming_mode {
        let errors = apply_optional_actions(options, false);
        if !errors.is_empty() {
            return (true, errors, true);
        }
    }

    let applied_state = set_gaming_mode_enabled(enable_gaming_mode);
    if applied_state != enable_gaming_mode {
        return (
            applied_state,
            vec!["Taskbar auto-hide: Windows did not apply the requested state".to_owned()],
            false,
        );
    }

    let errors = if enable_gaming_mode {
        apply_optional_actions(options, true)
    } else {
        Vec::new()
    };

    let optional_actions_failed = !errors.is_empty();
    (applied_state, errors, optional_actions_failed)
}

/// Toggles the main gaming state and reports action failures to the GUI.
#[cfg(feature = "gui")]
pub fn toggle(options: ToggleOptions) -> ToggleReport {
    let (gaming_mode_enabled, errors, optional_actions_failed) = apply_toggle(options);
    ToggleReport {
        gaming_mode_enabled,
        errors,
        optional_actions_failed,
    }
}

/// Preserves the original one-click behavior when the GUI feature is absent.
#[cfg(not(feature = "gui"))]
pub fn run() {
    let _ = apply_toggle(ToggleOptions::all());
}

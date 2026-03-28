use std::mem::size_of;

use windows::Win32::{
    Foundation::LPARAM,
    UI::Shell::{ABM_GETSTATE, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage},
};

#[cfg(feature = "desktop-icons")]
use windows::core::w;

#[cfg(feature = "desktop-icons")]
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        FindWindowExW, FindWindowW, IsWindowVisible, SW_HIDE, SW_SHOW, ShowWindow,
    },
};

#[cfg(any(feature = "desktop-background", feature = "minimize-all-windows"))]
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};

#[cfg(feature = "desktop-background")]
use windows::Win32::{
    Foundation::COLORREF,
    UI::Shell::{DesktopWallpaper, IDesktopWallpaper},
};

#[cfg(feature = "minimize-all-windows")]
use windows::Win32::UI::Shell::{IShellDispatch, Shell};

#[cfg(feature = "desktop-icons")]
fn find_desktop_def_view() -> Option<HWND> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), None).ok()?;
        if !progman.0.is_null() {
            let def_view = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None).ok()?;
            if !def_view.0.is_null() {
                return Some(def_view);
            }
        }

        let mut worker = HWND(std::ptr::null_mut());

        loop {
            worker = FindWindowExW(None, Some(worker), w!("WorkerW"), None).ok()?;
            if worker.0.is_null() {
                break;
            }

            let def_view = FindWindowExW(Some(worker), None, w!("SHELLDLL_DefView"), None).ok()?;
            if !def_view.0.is_null() {
                return Some(def_view);
            }
        }

        None
    }
}

#[cfg(feature = "desktop-icons")]
fn toggle_desktop_icons() {
    unsafe {
        let Some(def_view) = find_desktop_def_view() else {
            return;
        };

        let Ok(list_view) = FindWindowExW(Some(def_view), None, w!("SysListView32"), None) else {
            return;
        };

        if list_view.0.is_null() {
            return;
        }

        let is_visible = IsWindowVisible(list_view).as_bool();
        let _ = ShowWindow(list_view, if is_visible { SW_HIDE } else { SW_SHOW });
    }
}

#[cfg(feature = "desktop-background")]
fn apply_solid_background(enable_black_background: bool) {
    unsafe {
        if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
            return;
        }

        let result = (|| {
            let wallpaper: IDesktopWallpaper =
                CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)?;

            if enable_black_background {
                wallpaper.SetBackgroundColor(COLORREF(0x000000))?;
                wallpaper.Enable(false)?;
            } else {
                wallpaper.Enable(true)?;
            }

            Ok::<(), windows::core::Error>(())
        })();

        let _ = result;
        CoUninitialize();
    }
}

#[cfg(feature = "minimize-all-windows")]
fn minimize_all_windows() {
    unsafe {
        if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
            return;
        }

        let result = (|| {
            let shell: IShellDispatch = CoCreateInstance(&Shell, None, CLSCTX_ALL)?;
            shell.MinimizeAll()?;
            Ok::<(), windows::core::Error>(())
        })();

        let _ = result;
        CoUninitialize();
    }
}

pub fn run() {
    unsafe {
        let mut appbar = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            ..Default::default()
        };

        let current_state = SHAppBarMessage(ABM_GETSTATE, &mut appbar);
        let autohide_flag = ABS_AUTOHIDE as usize;
        let is_autohide_enabled = (current_state & autohide_flag) != 0;

        let toggled_state = if is_autohide_enabled {
            current_state & !autohide_flag
        } else {
            current_state | autohide_flag
        };

        appbar.lParam = LPARAM(toggled_state as isize);
        let _ = SHAppBarMessage(ABM_SETSTATE, &mut appbar);

        let did_enable_autohide = !is_autohide_enabled;

        #[cfg(feature = "desktop-icons")]
        toggle_desktop_icons();

        #[cfg(feature = "desktop-background")]
        apply_solid_background(did_enable_autohide);

        #[cfg(feature = "minimize-all-windows")]
        if did_enable_autohide {
            minimize_all_windows();
        }
    }
}

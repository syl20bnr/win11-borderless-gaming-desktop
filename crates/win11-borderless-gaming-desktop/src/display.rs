use std::{cmp::Reverse, collections::BTreeSet, fmt, mem::size_of};

use serde::{Deserialize, Serialize};
use windows::{
    Win32::{
        Devices::Display::{
            DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_PREFERRED_MODE, DISPLAYCONFIG_DEVICE_INFO_HEADER,
            DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_ROTATION_ROTATE90,
            DISPLAYCONFIG_ROTATION_ROTATE270, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
            DISPLAYCONFIG_TARGET_PREFERRED_MODE, DisplayConfigGetDeviceInfo,
            GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
        },
        Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, HWND},
        Graphics::Gdi::{
            CDS_TEST, CDS_TYPE, ChangeDisplaySettingsW, DEVMODEW, DISP_CHANGE,
            DISP_CHANGE_BADDUALVIEW, DISP_CHANGE_BADFLAGS, DISP_CHANGE_BADMODE,
            DISP_CHANGE_BADPARAM, DISP_CHANGE_FAILED, DISP_CHANGE_NOTUPDATED, DISP_CHANGE_RESTART,
            DISP_CHANGE_SUCCESSFUL, DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS,
            ENUM_DISPLAY_SETTINGS_MODE, EnumDisplaySettingsW, GetMonitorInfoW,
            MONITOR_DEFAULTTOPRIMARY, MONITORINFOEXW, MonitorFromWindow,
        },
    },
    core::PCWSTR,
};

/// A display resolution in physical pixels.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl fmt::Display for Resolution {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} × {}", self.width, self.height)
    }
}

/// The result of asking Windows to apply a supported resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeOutcome {
    Applied,
    RestartRequired,
}

/// A failure reported by `ChangeDisplaySettingsW`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeError {
    BadDualView,
    BadFlags,
    BadMode,
    BadParameter,
    Failed,
    NotUpdated,
    Unknown(i32),
}

impl fmt::Display for ChangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::BadDualView => "the display driver does not support this change in DualView mode",
            Self::BadFlags => "an invalid set of display-change flags was supplied",
            Self::BadMode => "the requested display mode is not supported",
            Self::BadParameter => "an invalid display parameter was supplied",
            Self::Failed => "the display driver rejected the change",
            Self::NotUpdated => "Windows did not update the display mode",
            Self::Unknown(code) => {
                return write!(formatter, "unknown display-change result ({code})");
            }
        };

        formatter.write_str(message)
    }
}

impl std::error::Error for ChangeError {}

/// An error encountered while reading or changing the primary display mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayError {
    NoResolutions,
    CurrentResolutionUnavailable,
    NativeResolutionUnavailable,
    DisplayConfigurationFailed(u32),
    ValidationFailed(ChangeError),
    ApplyFailed(ChangeError),
}

impl fmt::Display for DisplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoResolutions => formatter.write_str("Windows reported no primary-display modes"),
            Self::CurrentResolutionUnavailable => {
                formatter.write_str("Windows could not read the primary display's current mode")
            }
            Self::NativeResolutionUnavailable => formatter
                .write_str("Windows could not identify the primary display's preferred mode"),
            Self::DisplayConfigurationFailed(code) => {
                write!(
                    formatter,
                    "Windows display configuration query failed ({code})"
                )
            }
            Self::ValidationFailed(error) => {
                write!(
                    formatter,
                    "the requested resolution failed validation: {error}"
                )
            }
            Self::ApplyFailed(error) => {
                write!(
                    formatter,
                    "Windows could not apply the requested resolution: {error}"
                )
            }
        }
    }
}

impl std::error::Error for DisplayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ValidationFailed(error) | Self::ApplyFailed(error) => Some(error),
            Self::NoResolutions
            | Self::CurrentResolutionUnavailable
            | Self::NativeResolutionUnavailable
            | Self::DisplayConfigurationFailed(_) => None,
        }
    }
}

/// Returns the unique resolutions supported by the Windows primary display.
///
/// Resolutions are ordered by height first, then width, both largest to
/// smallest. Refresh-rate and color-depth variants are intentionally collapsed
/// for a compact picker.
pub fn primary_resolutions() -> Result<Vec<Resolution>, DisplayError> {
    let mut resolutions = Vec::new();
    let mut mode_number = 0_u32;

    loop {
        let mut mode = initialized_mode();

        // A null device name makes EnumDisplaySettingsW use the primary display.
        let found = unsafe {
            EnumDisplaySettingsW(
                PCWSTR::null(),
                ENUM_DISPLAY_SETTINGS_MODE(mode_number),
                &mut mode,
            )
        }
        .as_bool();

        if !found {
            break;
        }

        resolutions.push(Resolution::new(mode.dmPelsWidth, mode.dmPelsHeight));

        let Some(next_mode_number) = mode_number.checked_add(1) else {
            break;
        };
        mode_number = next_mode_number;
    }

    let resolutions = deduplicate_resolutions(resolutions);
    if resolutions.is_empty() {
        Err(DisplayError::NoResolutions)
    } else {
        Ok(resolutions)
    }
}

/// Returns the current resolution of the Windows primary display.
pub fn primary_resolution() -> Result<Resolution, DisplayError> {
    let mode = current_primary_mode()?;
    Ok(Resolution::new(mode.dmPelsWidth, mode.dmPelsHeight))
}

/// Returns the preferred (native) resolution of the primary monitor.
pub fn primary_native_resolution() -> Result<Resolution, DisplayError> {
    let primary_name = primary_gdi_device_name()?;
    let (paths, _) = active_display_configuration()?;
    let primary_path = paths
        .iter()
        .find(|path| source_gdi_device_name(path).is_some_and(|name| name == primary_name));

    let Some(path) = primary_path else {
        return Err(DisplayError::NativeResolutionUnavailable);
    };

    let mut preferred = DISPLAYCONFIG_TARGET_PREFERRED_MODE {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_PREFERRED_MODE,
            size: size_of::<DISPLAYCONFIG_TARGET_PREFERRED_MODE>() as u32,
            adapterId: path.targetInfo.adapterId,
            id: path.targetInfo.id,
        },
        ..Default::default()
    };

    let result = unsafe { DisplayConfigGetDeviceInfo(&mut preferred.header) };
    if result != ERROR_SUCCESS.0 as i32 || preferred.width == 0 || preferred.height == 0 {
        return Err(DisplayError::NativeResolutionUnavailable);
    }

    let rotated = path.targetInfo.rotation == DISPLAYCONFIG_ROTATION_ROTATE90
        || path.targetInfo.rotation == DISPLAYCONFIG_ROTATION_ROTATE270;
    let (width, height) = if rotated {
        (preferred.height, preferred.width)
    } else {
        (preferred.width, preferred.height)
    };

    Ok(Resolution::new(width, height))
}

/// Tests and then dynamically applies a resolution to the Windows primary display.
///
/// Only width and height are requested so Windows can retain or choose compatible
/// values for refresh rate, color depth, orientation, and the remaining mode fields.
pub fn set_primary_resolution(resolution: Resolution) -> Result<ChangeOutcome, DisplayError> {
    let mut mode = current_primary_mode()?;
    mode.dmPelsWidth = resolution.width;
    mode.dmPelsHeight = resolution.height;
    mode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;

    // The first call is non-mutating and asks the driver to validate the mode.
    let validation = unsafe { ChangeDisplaySettingsW(Some(&mode), CDS_TEST) };
    map_change_result(validation).map_err(DisplayError::ValidationFailed)?;

    // Zero flags apply the mode dynamically. The GUI persists its Desktop and
    // Gaming profiles itself, so the Gaming profile does not become Windows'
    // permanent default if the process exits while Gaming mode is active.
    let result = unsafe { ChangeDisplaySettingsW(Some(&mode), CDS_TYPE(0)) };
    map_change_result(result).map_err(DisplayError::ApplyFailed)
}

fn initialized_mode() -> DEVMODEW {
    DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    }
}

fn current_primary_mode() -> Result<DEVMODEW, DisplayError> {
    let mut mode = initialized_mode();

    // A null device name and ENUM_CURRENT_SETTINGS address the primary display.
    let found =
        unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut mode) }.as_bool();

    found
        .then_some(mode)
        .ok_or(DisplayError::CurrentResolutionUnavailable)
}

fn primary_gdi_device_name() -> Result<[u16; 32], DisplayError> {
    let monitor = unsafe { MonitorFromWindow(HWND::default(), MONITOR_DEFAULTTOPRIMARY) };
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;

    let found = unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.as_bool();
    found
        .then_some(info.szDevice)
        .ok_or(DisplayError::NativeResolutionUnavailable)
}

fn source_gdi_device_name(path: &DISPLAYCONFIG_PATH_INFO) -> Option<[u16; 32]> {
    let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            size: size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
            adapterId: path.sourceInfo.adapterId,
            id: path.sourceInfo.id,
        },
        ..Default::default()
    };

    let result = unsafe { DisplayConfigGetDeviceInfo(&mut source.header) };
    (result == ERROR_SUCCESS.0 as i32).then_some(source.viewGdiDeviceName)
}

fn active_display_configuration()
-> Result<(Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>), DisplayError> {
    for _ in 0..4 {
        let mut path_count = 0_u32;
        let mut mode_count = 0_u32;
        let result = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        };
        if result != ERROR_SUCCESS {
            return Err(DisplayError::DisplayConfigurationFailed(result.0));
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
        let result = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };

        if result == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if result != ERROR_SUCCESS {
            return Err(DisplayError::DisplayConfigurationFailed(result.0));
        }

        paths.truncate(path_count as usize);
        modes.truncate(mode_count as usize);
        return Ok((paths, modes));
    }

    Err(DisplayError::NativeResolutionUnavailable)
}

fn deduplicate_resolutions(resolutions: impl IntoIterator<Item = Resolution>) -> Vec<Resolution> {
    let mut resolutions = resolutions
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    sort_resolutions_descending(&mut resolutions);
    resolutions
}

pub(crate) fn sort_resolutions_descending(resolutions: &mut [Resolution]) {
    resolutions.sort_unstable_by_key(|resolution| Reverse((resolution.height, resolution.width)));
}

fn map_change_result(result: DISP_CHANGE) -> Result<ChangeOutcome, ChangeError> {
    match result {
        DISP_CHANGE_SUCCESSFUL => Ok(ChangeOutcome::Applied),
        DISP_CHANGE_RESTART => Ok(ChangeOutcome::RestartRequired),
        DISP_CHANGE_BADDUALVIEW => Err(ChangeError::BadDualView),
        DISP_CHANGE_BADFLAGS => Err(ChangeError::BadFlags),
        DISP_CHANGE_BADMODE => Err(ChangeError::BadMode),
        DISP_CHANGE_BADPARAM => Err(ChangeError::BadParameter),
        DISP_CHANGE_FAILED => Err(ChangeError::Failed),
        DISP_CHANGE_NOTUPDATED => Err(ChangeError::NotUpdated),
        other => Err(ChangeError::Unknown(other.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolutions_are_deduplicated_and_sorted_by_height_then_width_descending() {
        let resolutions = deduplicate_resolutions([
            Resolution::new(3840, 1080),
            Resolution::new(1280, 720),
            Resolution::new(2560, 1440),
            Resolution::new(3440, 1440),
            Resolution::new(3840, 1080),
            Resolution::new(1920, 1200),
        ]);

        assert_eq!(
            resolutions,
            [
                Resolution::new(3440, 1440),
                Resolution::new(2560, 1440),
                Resolution::new(1920, 1200),
                Resolution::new(3840, 1080),
                Resolution::new(1280, 720),
            ]
        );
    }

    #[test]
    fn resolution_has_a_picker_friendly_label() {
        assert_eq!(Resolution::new(2560, 1440).to_string(), "2560 × 1440");
    }

    #[test]
    fn successful_and_restart_results_are_outcomes() {
        assert_eq!(
            map_change_result(DISP_CHANGE_SUCCESSFUL),
            Ok(ChangeOutcome::Applied)
        );
        assert_eq!(
            map_change_result(DISP_CHANGE_RESTART),
            Ok(ChangeOutcome::RestartRequired)
        );
    }

    #[test]
    fn driver_failures_are_mapped_without_calling_windows() {
        assert_eq!(
            map_change_result(DISP_CHANGE_BADMODE),
            Err(ChangeError::BadMode)
        );
        assert_eq!(
            map_change_result(DISP_CHANGE_NOTUPDATED),
            Err(ChangeError::NotUpdated)
        );
        assert_eq!(
            map_change_result(DISP_CHANGE(42)),
            Err(ChangeError::Unknown(42))
        );
    }
}

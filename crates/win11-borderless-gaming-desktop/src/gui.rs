use std::{
    error::Error,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontFamily, FontId, Frame, Layout, Margin,
    RichText, Stroke, StrokeKind, TextStyle, Vec2, WidgetInfo, WidgetType,
};
use serde::{Deserialize, Serialize};
use tray_icon::{
    Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{ContextMenu, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use windows::Win32::UI::WindowsAndMessaging::{HMENU, SetMenuDefaultItem};

use crate::{
    app::{self, ToggleOptions},
    display::{self, ChangeOutcome, DisplayError, Resolution},
};

const APP_ID: &str = "win11-borderless-gaming-desktop";
const APP_TITLE: &str = "Borderless Gaming Desktop";
const STORAGE_KEY: &str = "gui-settings";
const EFRAME_WINDOW_STORAGE_KEY: &str = "window";
const EFRAME_MEMORY_STORAGE_KEY: &str = "egui";
const GAMING_COUNTDOWN_SECONDS: f64 = 4.0;
const TRAY_BLINK_HALF_PERIOD_SECONDS: f64 = 0.5;
const FIXED_SIZE_SETTLE_PASSES: u8 = 20;
const WORDMARK_TEXTURE_HEIGHT: u32 = 96;
const WORDMARK_DISPLAY_HEIGHT: f32 = 30.0;
const HEADER_HEIGHT: f32 = 50.0;
const HEADER_CLOSE_CLEARANCE: f32 = 24.0;
const WINDOW_WIDTH: f32 = 520.0;
// Measured from the rendered content so the resolution card keeps the same
// 24 px outer inset as the other three sides of the fixed window.
const WINDOW_BASE_HEIGHT: f32 = 475.0;
const COMPILED_ACTION_HEIGHT: f32 = 34.0;
const COMPILED_ACTION_COUNT: u8 = cfg!(feature = "desktop-icons") as u8
    + cfg!(feature = "desktop-background") as u8
    + cfg!(feature = "minimize-all-windows") as u8;
const WINDOW_SIZE: [f32; 2] = [WINDOW_WIDTH, fixed_window_height(COMPILED_ACTION_COUNT)];

const fn fixed_window_height(action_count: u8) -> f32 {
    WINDOW_BASE_HEIGHT + COMPILED_ACTION_HEIGHT * action_count as f32
}

const BACKGROUND: Color32 = Color32::from_rgb(12, 15, 22);
const CARD: Color32 = Color32::from_rgb(22, 27, 38);
const CARD_BORDER: Color32 = Color32::from_rgb(45, 54, 72);
const TEXT_MUTED: Color32 = Color32::from_rgb(151, 161, 180);
const ACCENT: Color32 = Color32::from_rgb(112, 98, 255);
const ACCENT_HOVER: Color32 = Color32::from_rgb(130, 117, 255);
// Keep these status colors in sync with the tray derivatives in `xtask`.
const GAMING_GREEN: Color32 = Color32::from_rgb(74, 222, 128);
const ACTIVATING_ORANGE: Color32 = Color32::from_rgb(249, 151, 71);
const LED_OFF: Color32 = Color32::from_rgb(148, 156, 181);

pub fn run() -> eframe::Result {
    let mut viewport = fixed_viewport(egui::ViewportBuilder::default().with_app_id(APP_ID));

    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        centered: true,
        // App settings are persisted explicitly. Native geometry never is.
        persist_window: false,
        // Eframe loads legacy geometry even when `persist_window` is false. This
        // late hook runs after that load and overwrites every mutable size flag.
        window_builder: Some(Box::new(fixed_viewport)),
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        native_options,
        Box::new(|creation_context| Ok(Box::new(GuiApp::new(creation_context)?))),
    )
}

fn fixed_viewport(mut viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
    let fixed_size = Vec2::from(WINDOW_SIZE);

    // Eframe centers using the restored size before invoking `window_builder`.
    // Correct that position by the size delta while replacing stale geometry.
    if let (Some(position), Some(restored_size)) = (viewport.position, viewport.inner_size) {
        viewport.position = Some(position + (restored_size - fixed_size) * 0.5);
    }

    viewport
        .with_inner_size(fixed_size)
        .with_min_inner_size(fixed_size)
        .with_max_inner_size(fixed_size)
        .with_clamp_size_to_monitor_size(false)
        .with_fullscreen(false)
        .with_maximized(false)
        .with_decorations(false)
        .with_resizable(false)
        .with_maximize_button(false)
}

fn window_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory_with_format(
        include_bytes!("../assets/app.ico"),
        image::ImageFormat::Ico,
    )
    .ok()?
    .into_rgba8();
    let (width, height) = image.dimensions();

    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct FeatureChoices {
    #[cfg(feature = "desktop-icons")]
    desktop_icons: bool,
    #[cfg(feature = "desktop-background")]
    desktop_background: bool,
    #[cfg(feature = "minimize-all-windows")]
    minimize_all_windows: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for FeatureChoices {
    fn default() -> Self {
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

impl FeatureChoices {
    fn toggle_options(&self) -> ToggleOptions {
        ToggleOptions {
            #[cfg(feature = "desktop-icons")]
            desktop_icons: self.desktop_icons,
            #[cfg(feature = "desktop-background")]
            desktop_background: self.desktop_background,
            #[cfg(feature = "minimize-all-windows")]
            minimize_all_windows: self.minimize_all_windows,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct Settings {
    features: FeatureChoices,
    active_mode_features: Option<FeatureChoices>,
    desktop_resolution: Option<Resolution>,
    gaming_resolution: Option<Resolution>,
    tray_close_notice_acknowledged: bool,
}

#[derive(Clone, Copy, Debug)]
enum TrayCommand {
    Open,
    ToggleMode,
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModeVisualState {
    Desktop,
    Activating(u8),
    Gaming,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrayIconState {
    Desktop,
    ActivatingBright,
    ActivatingDim,
    Gaming,
}

#[derive(Clone, Copy)]
enum AnimatedButtonTone {
    Primary,
    Secondary,
}

#[derive(Default)]
struct DisplayPicker {
    resolutions: Vec<Resolution>,
    available_resolutions: Vec<Resolution>,
    native: Option<Resolution>,
}

struct ModeWordmark {
    texture: egui::TextureHandle,
    aspect_ratio: f32,
}

struct ModeWordmarks {
    desktop: ModeWordmark,
    gaming: ModeWordmark,
}

impl ModeWordmarks {
    fn load(context: &egui::Context) -> Result<Self, image::ImageError> {
        Ok(Self {
            desktop: load_mode_wordmark(
                context,
                "desktop-mode-wordmark",
                include_bytes!("../assets/desktop-mode-wordmark.png"),
            )?,
            gaming: load_mode_wordmark(
                context,
                "gaming-mode-wordmark",
                include_bytes!("../assets/gaming-mode-wordmark.png"),
            )?,
        })
    }

    fn for_state(&self, state: ModeVisualState) -> &ModeWordmark {
        match state {
            ModeVisualState::Desktop => &self.desktop,
            ModeVisualState::Activating(_) | ModeVisualState::Gaming => &self.gaming,
        }
    }
}

fn load_mode_wordmark(
    context: &egui::Context,
    name: &str,
    bytes: &[u8],
) -> Result<ModeWordmark, image::ImageError> {
    let source = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.into_rgba8();
    let (source_width, source_height) = source.dimensions();

    let (left, top, right, bottom) = alpha_bounds(&source).unwrap_or((
        0,
        0,
        source_width.saturating_sub(1),
        source_height.saturating_sub(1),
    ));
    let cropped = image::imageops::crop_imm(
        &source,
        left,
        top,
        right.saturating_sub(left) + 1,
        bottom.saturating_sub(top) + 1,
    )
    .to_image();
    let target_width = ((cropped.width() as f32 / cropped.height() as f32)
        * WORDMARK_TEXTURE_HEIGHT as f32)
        .round()
        .max(1.0) as u32;
    let resized = image::imageops::resize(
        &cropped,
        target_width,
        WORDMARK_TEXTURE_HEIGHT,
        image::imageops::FilterType::Lanczos3,
    );
    let size = [resized.width() as usize, resized.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, resized.as_raw());
    let texture = context.load_texture(name, color_image, egui::TextureOptions::LINEAR);

    Ok(ModeWordmark {
        texture,
        aspect_ratio: size[0] as f32 / size[1] as f32,
    })
}

fn alpha_bounds(image: &image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (width, height) = image.dimensions();
    let mut left = width;
    let mut top = height;
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;

    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] > 8 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
            found = true;
        }
    }

    found.then(|| {
        const PADDING: u32 = 6;
        (
            left.saturating_sub(PADDING),
            top.saturating_sub(PADDING),
            right.saturating_add(PADDING).min(width.saturating_sub(1)),
            bottom.saturating_add(PADDING).min(height.saturating_sub(1)),
        )
    })
}

impl DisplayPicker {
    fn load(&mut self, settings: &mut Settings) -> Result<(), DisplayError> {
        self.resolutions.clear();
        self.available_resolutions.clear();
        self.native = None;

        let mut resolutions = display::primary_resolutions()?;
        let current = display::primary_resolution()?;
        let native = display::primary_native_resolution().ok();

        resolutions.retain(|resolution| resolution_is_allowed(*resolution, native));

        for resolution in [Some(current), native].into_iter().flatten() {
            if resolution_is_allowed(resolution, native) && !resolutions.contains(&resolution) {
                resolutions.push(resolution);
            }
        }
        display::sort_resolutions_descending(&mut resolutions);
        resolutions.dedup();

        let default = if resolution_is_allowed(current, native) {
            current
        } else {
            native
                .or_else(|| resolutions.first().copied())
                .ok_or(DisplayError::NoResolutions)?
        };

        self.available_resolutions = resolutions.clone();
        self.native = native;
        initialize_profile(&mut settings.desktop_resolution, default);
        initialize_profile(&mut settings.gaming_resolution, default);
        replace_disallowed_profile(&mut settings.desktop_resolution, default, native);
        replace_disallowed_profile(&mut settings.gaming_resolution, default, native);

        // Preserve profiles that are temporarily unavailable (for example under
        // Remote Desktop or while another monitor is connected). They remain
        // visible but disabled instead of being silently overwritten on save.
        for resolution in [settings.desktop_resolution, settings.gaming_resolution]
            .into_iter()
            .flatten()
        {
            if resolution_is_allowed(resolution, native) && !resolutions.contains(&resolution) {
                resolutions.push(resolution);
            }
        }
        display::sort_resolutions_descending(&mut resolutions);
        resolutions.dedup();
        self.resolutions = resolutions;
        Ok(())
    }
}

fn resolution_is_allowed(resolution: Resolution, native: Option<Resolution>) -> bool {
    native.is_none_or(|native| resolution.height <= native.height)
}

fn replace_disallowed_profile(
    profile: &mut Option<Resolution>,
    default: Resolution,
    native: Option<Resolution>,
) {
    if profile.is_some_and(|resolution| !resolution_is_allowed(resolution, native)) {
        *profile = Some(default);
    }
}

fn initialize_profile(profile: &mut Option<Resolution>, default: Resolution) {
    if profile.is_none() {
        *profile = Some(default);
    }
}

struct GuiApp {
    settings: Settings,
    display: DisplayPicker,
    mode_wordmarks: ModeWordmarks,
    errors: Vec<String>,
    pending_resolution: Option<(Resolution, String)>,
    tray_commands: Receiver<TrayCommand>,
    tray_icon: TrayIcon,
    tray_icon_state: TrayIconState,
    tray_mode_item: MenuItem,
    tray_menu_state: ModeVisualState,
    activation_started_at: Option<f64>,
    close_notice_open: bool,
    fixed_size_settle_passes: u8,
    restore_actions_failed: bool,
    quitting: bool,
}

impl GuiApp {
    fn new(
        creation_context: &eframe::CreationContext<'_>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // No egui interaction/layout state is persistent. In particular, never
        // restore a scroll offset captured from an iconic 160x64 viewport.
        creation_context
            .egui_ctx
            .memory_mut(|memory| *memory = egui::Memory::default());
        configure_style(&creation_context.egui_ctx);
        let mode_wordmarks = ModeWordmarks::load(&creation_context.egui_ctx)?;

        let mut settings: Settings = creation_context
            .storage
            .and_then(|storage| eframe::get_value(storage, STORAGE_KEY))
            .unwrap_or_default();

        let gaming_mode = app::gaming_mode_enabled();
        if !gaming_mode {
            settings.active_mode_features = None;
        }

        let mut display = DisplayPicker::default();
        let errors = display
            .load(&mut settings)
            .err()
            .map(|error| vec![format!("Display modes are unavailable: {error}")])
            .unwrap_or_default();

        let mode_state = if gaming_mode {
            ModeVisualState::Gaming
        } else {
            ModeVisualState::Desktop
        };
        let tray_icon_state = if gaming_mode {
            TrayIconState::Gaming
        } else {
            TrayIconState::Desktop
        };
        let (tray_icon, tray_mode_item, tray_commands) =
            create_tray(&creation_context.egui_ctx, tray_icon_state, mode_state)?;

        Ok(Self {
            settings,
            display,
            mode_wordmarks,
            errors,
            pending_resolution: None,
            tray_commands,
            tray_icon,
            tray_icon_state,
            tray_mode_item,
            tray_menu_state: mode_state,
            activation_started_at: None,
            close_notice_open: false,
            fixed_size_settle_passes: 0,
            restore_actions_failed: false,
            quitting: false,
        })
    }

    fn toggle_mode(&mut self, context: &egui::Context) {
        self.errors.clear();
        self.pending_resolution = None;
        let was_enabled = app::gaming_mode_enabled();
        let choices = if was_enabled {
            self.settings
                .active_mode_features
                .clone()
                .unwrap_or_else(|| self.settings.features.clone())
        } else {
            let choices = self.settings.features.clone();
            self.settings.active_mode_features = Some(choices.clone());
            choices
        };

        let report = app::toggle(choices.toggle_options());
        let transition_succeeded = report.gaming_mode_enabled != was_enabled;
        self.restore_actions_failed =
            was_enabled && report.gaming_mode_enabled && report.optional_actions_failed;

        if report.gaming_mode_enabled {
            self.settings.active_mode_features = Some(choices);
        } else {
            self.settings.active_mode_features = None;
        }

        let mut errors = report.errors;
        if !transition_succeeded {
            if self.restore_actions_failed {
                errors.push(
                    "Uncheck a failing action if Windows cannot restore it, then retry.".to_owned(),
                );
            } else if errors.is_empty() {
                errors.push("Windows did not change the main mode.".to_owned());
            }
        } else {
            let target = if report.gaming_mode_enabled {
                self.settings.gaming_resolution
            } else {
                self.settings.desktop_resolution
            };

            if let Some(target) = target
                && let Err(error) = self.apply_profile_resolution(target)
            {
                self.pending_resolution = Some((target, error));
            }
        }

        self.errors = errors;
        // ChangeDisplaySettingsW can synchronously alter the viewport placement.
        // Reassert the immutable dimensions now and again after Windows settles.
        self.begin_fixed_size_settle(context);
    }

    fn begin_fixed_size_settle(&mut self, context: &egui::Context) {
        self.fixed_size_settle_passes = FIXED_SIZE_SETTLE_PASSES;
        request_fixed_window_size(context);
        context.request_repaint();
    }

    fn visual_state(&self, gaming_mode: bool, now: f64) -> ModeVisualState {
        if gaming_mode {
            ModeVisualState::Gaming
        } else if let Some(started_at) = self.activation_started_at {
            ModeVisualState::Activating(countdown_digit(started_at, now))
        } else {
            ModeVisualState::Desktop
        }
    }

    fn request_mode_toggle(&mut self, context: &egui::Context, now: f64) {
        if app::gaming_mode_enabled() {
            self.activation_started_at = None;
            self.toggle_mode(context);
        } else if self.activation_started_at.is_none() {
            self.errors.clear();
            self.pending_resolution = None;
            self.activation_started_at = Some(now);
            context.request_repaint();
        }
    }

    fn request_close_to_tray(&mut self, context: &egui::Context) {
        if self.settings.tray_close_notice_acknowledged {
            hide_to_tray(context);
        } else {
            self.close_notice_open = true;
            context.request_repaint();
        }
    }

    fn desired_tray_icon_state(&self, state: ModeVisualState, now: f64) -> TrayIconState {
        match state {
            ModeVisualState::Desktop => TrayIconState::Desktop,
            ModeVisualState::Gaming => TrayIconState::Gaming,
            ModeVisualState::Activating(_) => activating_tray_icon_state(
                self.activation_started_at
                    .map_or(0.0, |started_at| now - started_at),
            ),
        }
    }

    fn sync_tray_icon(&mut self, state: TrayIconState) {
        if state == self.tray_icon_state {
            return;
        }

        let result = tray_status_icon(state)
            .and_then(|icon| self.tray_icon.set_icon(Some(icon)).map_err(Into::into));
        match result {
            Ok(()) => self.tray_icon_state = state,
            Err(error) => {
                let message = format!("Could not update the system-tray status icon: {error}");
                if !self.errors.contains(&message) {
                    self.errors.push(message);
                }
            }
        }
    }

    fn sync_tray_menu(&mut self, state: ModeVisualState) {
        if state == self.tray_menu_state {
            return;
        }

        let (label, enabled) = tray_mode_menu_presentation(state);
        self.tray_mode_item.set_text(label);
        self.tray_mode_item.set_enabled(enabled);
        self.tray_menu_state = state;
    }

    fn apply_profile_resolution(&self, target: Resolution) -> Result<(), String> {
        let current = display::primary_resolution()
            .map_err(|error| format!("Could not read the current resolution: {error}"))?;
        if current == target {
            return Ok(());
        }

        match display::set_primary_resolution(target) {
            Ok(ChangeOutcome::Applied) => {
                let applied = display::primary_resolution()
                    .map_err(|error| format!("Could not verify {target}: {error}"))?;
                if applied == target {
                    Ok(())
                } else {
                    Err(format!(
                        "Windows kept the primary monitor at {applied} instead of {target}."
                    ))
                }
            }
            Ok(ChangeOutcome::RestartRequired) => Err(format!(
                "Windows accepted {target}, but a restart is required before it can be used."
            )),
            Err(error) => Err(format!("Could not apply {target}: {error}")),
        }
    }

    fn show_header(&self, ui: &mut egui::Ui, state: ModeVisualState) {
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), HEADER_HEIGHT),
            egui::Sense::hover(),
        );
        let wordmark = self.mode_wordmarks.for_state(state);
        let wordmark_size = Vec2::new(
            wordmark.aspect_ratio * WORDMARK_DISPLAY_HEIGHT,
            WORDMARK_DISPLAY_HEIGHT,
        );
        let status_width = 20.0 + 8.0 + wordmark_size.x;
        let status_right = rect.right() - HEADER_CLOSE_CLEARANCE;
        let status_rect = egui::Rect::from_min_size(
            egui::pos2(
                status_right - status_width,
                rect.center().y - wordmark_size.y * 0.5,
            ),
            Vec2::new(status_width, wordmark_size.y),
        );
        let title_rect = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(status_rect.left() - 12.0, rect.bottom()),
        );

        let mut title_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(title_rect)
                .layout(Layout::top_down(Align::LEFT)),
        );
        title_ui.spacing_mut().item_spacing.y = 3.0;
        title_ui.label(
            RichText::new("Borderless Gaming")
                .size(24.0)
                .strong()
                .color(Color32::WHITE),
        );
        title_ui.label(
            RichText::new("The perfect gaming XP in one click")
                .size(13.0)
                .color(TEXT_MUTED),
        );

        let mut status_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(status_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        status_ui.spacing_mut().item_spacing.x = 8.0;
        mode_led(&mut status_ui, state);
        status_ui
            .add(
                egui::Image::new(&wordmark.texture)
                    .fit_to_exact_size(wordmark_size)
                    .alt_text(match state {
                        ModeVisualState::Desktop => "Desktop mode",
                        ModeVisualState::Activating(_) => "Gaming mode activating",
                        ModeVisualState::Gaming => "Gaming mode",
                    }),
            )
            .on_hover_text(mode_status_description(state));
    }

    fn show_mode_button(&mut self, ui: &mut egui::Ui, state: ModeVisualState) {
        let (label, spinner) = match state {
            ModeVisualState::Desktop => ("Enter gaming mode".to_owned(), false),
            ModeVisualState::Activating(countdown) => (format!("Activating in {countdown}"), true),
            ModeVisualState::Gaming => ("Restore desktop mode".to_owned(), false),
        };

        let enabled = !matches!(state, ModeVisualState::Activating(_));
        let width = ui.available_width();
        let response = ui
            .add_enabled_ui(enabled, |ui| {
                animated_text_button(
                    ui,
                    &label,
                    Vec2::new(width, 48.0),
                    AnimatedButtonTone::Primary,
                    spinner,
                )
            })
            .inner;

        if response.clicked() {
            self.request_mode_toggle(ui.ctx(), ui.input(|input| input.time));
        }
    }

    fn show_feature_choices(&mut self, ui: &mut egui::Ui, gaming_mode: bool) {
        #[cfg(not(any(
            feature = "desktop-icons",
            feature = "desktop-background",
            feature = "minimize-all-windows"
        )))]
        let _ = gaming_mode;

        card(ui, |ui| {
            ui.label(RichText::new("Gaming mode actions").size(15.0).strong());
            ui.add_space(4.0);

            #[cfg(not(any(
                feature = "desktop-icons",
                feature = "desktop-background",
                feature = "minimize-all-windows"
            )))]
            ui.label(
                RichText::new("This build changes taskbar auto-hide only.")
                    .size(13.0)
                    .color(TEXT_MUTED),
            );

            #[cfg(any(
                feature = "desktop-icons",
                feature = "desktop-background",
                feature = "minimize-all-windows"
            ))]
            ui.add_enabled_ui(!gaming_mode || self.restore_actions_failed, |ui| {
                #[cfg(feature = "desktop-icons")]
                accent_checkbox(
                    ui,
                    &mut self.settings.features.desktop_icons,
                    "Hide desktop icons",
                );
                #[cfg(feature = "desktop-background")]
                accent_checkbox(
                    ui,
                    &mut self.settings.features.desktop_background,
                    "Use a solid black desktop background",
                );
                #[cfg(feature = "minimize-all-windows")]
                accent_checkbox(
                    ui,
                    &mut self.settings.features.minimize_all_windows,
                    "Minimize open windows when entering",
                );
            });

            #[cfg(any(
                feature = "desktop-icons",
                feature = "desktop-background",
                feature = "minimize-all-windows"
            ))]
            if gaming_mode && self.restore_actions_failed {
                ui.add_space(3.0);
                ui.label(
                    RichText::new("Adjust the failing action, then retry desktop restore.")
                        .size(12.0)
                        .color(TEXT_MUTED),
                );
            }

            #[cfg(any(
                feature = "desktop-icons",
                feature = "desktop-background",
                feature = "minimize-all-windows"
            ))]
            if gaming_mode && self.restore_actions_failed {
                self.settings.active_mode_features = Some(self.settings.features.clone());
            }
        });
    }

    fn show_resolution_profiles(&mut self, ui: &mut egui::Ui) {
        let previous_desktop = self.settings.desktop_resolution;
        let previous_gaming = self.settings.gaming_resolution;
        let resolutions = self.display.resolutions.clone();
        let available_resolutions = self.display.available_resolutions.clone();
        let native = self.display.native;

        card(ui, |ui| {
            ui.label(RichText::new("Resolution profiles").size(15.0).strong());
            ui.label(
                RichText::new("Applied only when switching modes.")
                    .size(12.0)
                    .color(TEXT_MUTED),
            );
            ui.add_space(7.0);

            if resolutions.is_empty() {
                ui.label(
                    RichText::new("No primary-monitor resolutions are available.")
                        .color(Color32::from_rgb(248, 113, 113)),
                );
            } else if ui.available_width() >= 360.0 {
                ui.columns(2, |columns| {
                    resolution_profile_list(
                        &mut columns[0],
                        "Desktop mode",
                        "desktop-resolution-list",
                        &resolutions,
                        &available_resolutions,
                        native,
                        &mut self.settings.desktop_resolution,
                    );
                    resolution_profile_list(
                        &mut columns[1],
                        "Gaming mode",
                        "gaming-resolution-list",
                        &resolutions,
                        &available_resolutions,
                        native,
                        &mut self.settings.gaming_resolution,
                    );
                });
            } else {
                resolution_profile_list(
                    ui,
                    "Desktop mode",
                    "desktop-resolution-list-compact",
                    &resolutions,
                    &available_resolutions,
                    native,
                    &mut self.settings.desktop_resolution,
                );
                ui.add_space(10.0);
                resolution_profile_list(
                    ui,
                    "Gaming mode",
                    "gaming-resolution-list-compact",
                    &resolutions,
                    &available_resolutions,
                    native,
                    &mut self.settings.gaming_resolution,
                );
            }
        });

        if self.settings.desktop_resolution != previous_desktop
            || self.settings.gaming_resolution != previous_gaming
        {
            self.errors.clear();
            self.pending_resolution = None;
        }
    }

    fn show_errors(&mut self, ui: &mut egui::Ui) {
        if !self.errors.is_empty() {
            ui.horizontal_top(|ui| {
                ui.label(
                    RichText::new("!")
                        .strong()
                        .color(Color32::from_rgb(248, 113, 113)),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(self.errors.join("\n"))
                            .size(12.5)
                            .color(Color32::from_rgb(248, 170, 170)),
                    )
                    .wrap(),
                );
            });
        }

        if let Some((target, error)) = self.pending_resolution.clone() {
            if !self.errors.is_empty() {
                ui.add_space(5.0);
            }
            ui.horizontal_top(|ui| {
                ui.label(
                    RichText::new("!")
                        .strong()
                        .color(Color32::from_rgb(248, 113, 113)),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(error)
                            .size(12.5)
                            .color(Color32::from_rgb(248, 170, 170)),
                    )
                    .wrap(),
                );
            });
            ui.add_space(6.0);
            if animated_text_button(
                ui,
                "Retry resolution",
                Vec2::new(138.0, 30.0),
                AnimatedButtonTone::Secondary,
                false,
            )
            .clicked()
            {
                self.pending_resolution = self
                    .apply_profile_resolution(target)
                    .err()
                    .map(|error| (target, error));
                self.begin_fixed_size_settle(ui.ctx());
            }
        }
    }

    fn show_close_notice(&mut self, context: &egui::Context) {
        if !self.close_notice_open {
            return;
        }

        let accepted = egui::Modal::new(egui::Id::new("tray-close-notice"))
            .backdrop_color(Color32::from_black_alpha(196))
            .frame(
                Frame::new()
                    .fill(CARD)
                    .stroke(Stroke::new(1.0, ACCENT.gamma_multiply(0.72)))
                    .corner_radius(CornerRadius::same(16))
                    .inner_margin(Margin::same(24)),
            )
            .show(context, |ui| {
                ui.set_width(326.0);
                ui.vertical_centered(|ui| {
                    let (icon_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
                    ui.painter().circle_filled(
                        icon_rect.center(),
                        21.0,
                        Color32::from_rgba_unmultiplied(112, 98, 255, 34),
                    );
                    ui.painter()
                        .circle_stroke(icon_rect.center(), 15.0, Stroke::new(1.5, ACCENT));
                    ui.painter().text(
                        icon_rect.center(),
                        Align2::CENTER_CENTER,
                        "i",
                        FontId::new(20.0, FontFamily::Proportional),
                        Color32::WHITE,
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Minimized to the tray")
                            .size(19.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "The application won't close, it will minimize to the system tray.",
                            )
                            .size(13.5)
                            .color(TEXT_MUTED),
                        )
                        .wrap(),
                    );
                    ui.add_space(18.0);
                    let width = ui.available_width();
                    animated_text_button(
                        ui,
                        "OK",
                        Vec2::new(width, 40.0),
                        AnimatedButtonTone::Primary,
                        false,
                    )
                    .clicked()
                })
                .inner
            })
            .inner;

        if accepted {
            self.settings.tray_close_notice_acknowledged = true;
            self.close_notice_open = false;
            hide_to_tray(context);
        }
    }
}

impl eframe::App for GuiApp {
    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        let now = context.input(|input| input.time);

        while let Ok(command) = self.tray_commands.try_recv() {
            match command {
                TrayCommand::Open => {
                    context.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    context.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                    context.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    request_fixed_window_size(context);
                    context.send_viewport_cmd(egui::ViewportCommand::Focus);
                    self.begin_fixed_size_settle(context);
                }
                TrayCommand::ToggleMode => self.request_mode_toggle(context, now),
                TrayCommand::Quit => {
                    self.quitting = true;
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        let mut gaming_mode = app::gaming_mode_enabled();

        if gaming_mode {
            self.activation_started_at = None;
        } else if let Some(started_at) = self.activation_started_at {
            if now - started_at >= GAMING_COUNTDOWN_SECONDS {
                self.activation_started_at = None;
                self.toggle_mode(context);
                gaming_mode = app::gaming_mode_enabled();
            } else {
                context.request_repaint_after(Duration::from_millis(16));
            }
        }

        let state = self.visual_state(gaming_mode, now);
        let tray_icon_state = self.desired_tray_icon_state(state, now);
        self.sync_tray_icon(tray_icon_state);
        self.sync_tray_menu(state);

        if self.fixed_size_settle_passes > 0 {
            request_fixed_window_size(context);
            self.fixed_size_settle_passes -= 1;
            context.request_repaint_after(Duration::from_millis(16));
        }

        let size_changed_while_visible = context.input(|input| {
            let viewport = input.viewport();
            viewport.minimized != Some(true)
                && viewport
                    .inner_rect
                    .is_some_and(|rect| !is_fixed_window_size(rect.size()))
        });
        if size_changed_while_visible {
            request_fixed_window_size(context);
        }

        let close_requested = context.input(|input| input.viewport().close_requested());

        if close_requested && !self.quitting {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request_close_to_tray(context);
        }

        // Keep the LED accurate if taskbar auto-hide is changed outside this app.
        context.request_repaint_after(Duration::from_secs(1));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Register this before the controls so they keep interaction priority
        // over the full-window drag surface.
        let drag_response = ui.interact(
            ui.max_rect(),
            ui.make_persistent_id("window-drag-background"),
            egui::Sense::click_and_drag(),
        );
        if drag_response.drag_started_by(egui::PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let gaming_mode = app::gaming_mode_enabled();
        let state = self.visual_state(gaming_mode, ui.input(|input| input.time));

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BACKGROUND).inner_margin(Margin::same(24)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("main-controls")
                    .max_height(ui.available_height())
                    .auto_shrink([false, false])
                    // A minimized native window can briefly report a zero-sized
                    // viewport and make egui animate a stale scrollbar on restore.
                    // Keep scrolling available for small screens without showing
                    // that transient outer-page bar.
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.show_header(ui, state);
                        ui.add_space(8.0);
                        self.show_mode_button(ui, state);
                        if !self.errors.is_empty() || self.pending_resolution.is_some() {
                            ui.add_space(9.0);
                            self.show_errors(ui);
                        }
                        ui.add_space(9.0);
                        self.show_feature_choices(ui, gaming_mode);
                        ui.add_space(9.0);
                        self.show_resolution_profiles(ui);
                    });
            });

        if window_close_button(ui.ctx()) {
            self.request_close_to_tray(ui.ctx());
        }

        self.show_close_notice(ui.ctx());
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Migrate away from eframe's native geometry/UI persistence. These keys
        // may contain the 160x64 iconic size and its associated scroll state.
        storage.remove_string(EFRAME_WINDOW_STORAGE_KEY);
        storage.remove_string(EFRAME_MEMORY_STORAGE_KEY);
        eframe::set_value(storage, STORAGE_KEY, &self.settings);
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(5)
    }
}

fn request_fixed_window_size(context: &egui::Context) {
    let size = Vec2::from(WINDOW_SIZE);
    context.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(size));
    context.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(size));
    context.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
    context.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
}

fn hide_to_tray(context: &egui::Context) {
    // A tray-hidden window is explicitly restored out of the iconic state so
    // Windows cannot retain the 160x64 minimized geometry. Visibility alone
    // removes it from the taskbar; ordinary native minimization is untouched.
    context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    context.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
}

fn is_fixed_window_size(size: Vec2) -> bool {
    const EPSILON: f32 = 0.5;
    let expected = Vec2::from(WINDOW_SIZE);
    (size.x - expected.x).abs() <= EPSILON && (size.y - expected.y).abs() <= EPSILON
}

fn configure_style(context: &egui::Context) {
    context.options_mut(|options| options.zoom_with_keyboard = false);
    context.set_zoom_factor(1.0);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = CARD;
    visuals.extreme_bg_color = Color32::from_rgb(10, 13, 20);
    visuals.faint_bg_color = Color32::from_rgb(28, 34, 46);
    visuals.selection.bg_fill = ACCENT;
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(50, 58, 78);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);
    visuals.widgets.active.bg_fill = ACCENT_HOVER;
    visuals.widgets.open.corner_radius = CornerRadius::same(8);
    context.set_visuals(visuals);

    context.style_mut_of(egui::Theme::Dark, |style| {
        style.animation_time = 0.18;
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(14.0, 8.0);
        style.spacing.interact_size = Vec2::new(32.0, 28.0);
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        );
    });
}

fn create_tray(
    context: &egui::Context,
    icon_state: TrayIconState,
    mode_state: ModeVisualState,
) -> Result<(TrayIcon, MenuItem, Receiver<TrayCommand>), Box<dyn Error + Send + Sync>> {
    let menu = Menu::new();
    let open_item = MenuItem::new("Open", true, None);
    let (mode_label, mode_enabled) = tray_mode_menu_presentation(mode_state);
    let mode_item = MenuItem::new(mode_label, mode_enabled, None);
    let open_separator = PredefinedMenuItem::separator();
    let quit_separator = PredefinedMenuItem::separator();
    let quit_item = MenuItem::new("Quit", true, None);
    menu.append_items(&[
        &open_item,
        &open_separator,
        &mode_item,
        &quit_separator,
        &quit_item,
    ])?;
    unsafe {
        SetMenuDefaultItem(HMENU(menu.hpopupmenu() as *mut core::ffi::c_void), 0, 1)?;
    }

    let tray_icon = TrayIconBuilder::new()
        .with_tooltip(APP_TITLE)
        .with_icon(tray_status_icon(icon_state)?)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_menu_on_right_click(true)
        .build()?;

    let (sender, receiver) = mpsc::channel();

    let open_id = open_item.id().clone();
    let mode_id = mode_item.id().clone();
    let quit_id = quit_item.id().clone();
    let menu_sender = sender.clone();
    let menu_context = context.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let command = if event.id == open_id {
            Some(TrayCommand::Open)
        } else if event.id == mode_id {
            Some(TrayCommand::ToggleMode)
        } else if event.id == quit_id {
            Some(TrayCommand::Quit)
        } else {
            None
        };

        if let Some(command) = command {
            let _ = menu_sender.send(command);
            menu_context.request_repaint();
        }
    }));

    let tray_context = context.clone();
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if matches!(
            event,
            TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            }
        ) {
            let _ = sender.send(TrayCommand::Open);
            tray_context.request_repaint();
        }
    }));

    Ok((tray_icon, mode_item, receiver))
}

fn tray_status_icon(state: TrayIconState) -> Result<Icon, Box<dyn Error + Send + Sync>> {
    let bytes = match state {
        TrayIconState::Desktop => include_bytes!("../assets/tray-desktop.png").as_slice(),
        TrayIconState::ActivatingBright => {
            include_bytes!("../assets/tray-activating.png").as_slice()
        }
        TrayIconState::ActivatingDim => {
            include_bytes!("../assets/tray-activating-dim.png").as_slice()
        }
        TrayIconState::Gaming => include_bytes!("../assets/tray-gaming.png").as_slice(),
    };
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.into_rgba8();
    let (width, height) = image.dimensions();
    Ok(Icon::from_rgba(image.into_raw(), width, height)?)
}

fn resolution_profile_list(
    ui: &mut egui::Ui,
    title: &str,
    id: &str,
    resolutions: &[Resolution],
    available_resolutions: &[Resolution],
    native: Option<Resolution>,
    selection: &mut Option<Resolution>,
) {
    ui.label(RichText::new(title).size(13.0).strong());
    ui.add_space(3.0);

    Frame::new()
        .fill(BACKGROUND.gamma_multiply(0.72))
        .stroke(Stroke::new(1.0, CARD_BORDER.gamma_multiply(0.75)))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(4))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt(id)
                .max_height(98.0)
                .min_scrolled_height(98.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for resolution in resolutions.iter().copied() {
                        let available = available_resolutions.contains(&resolution);
                        let label = if native == Some(resolution) {
                            format!("{resolution} (native)")
                        } else if !available {
                            format!("{resolution} (unavailable)")
                        } else {
                            resolution.to_string()
                        };
                        let selected = *selection == Some(resolution);
                        let mut response = ui
                            .push_id((id, resolution), |ui| {
                                ui.add_enabled_ui(available, |ui| {
                                    animated_selectable_row(ui, &label, selected)
                                })
                                .inner
                            })
                            .inner;

                        if response.clicked() {
                            *selection = Some(resolution);
                            response.mark_changed();
                        }
                    }
                });
        });
}

fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, CARD_BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(15))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });
}

#[cfg(any(
    feature = "desktop-icons",
    feature = "desktop-background",
    feature = "minimize-all-windows"
))]
fn accent_checkbox(ui: &mut egui::Ui, checked: &mut bool, label: &str) -> egui::Response {
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), egui::Sense::click());

    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }

    let selected = *checked;
    response.widget_info(|| {
        WidgetInfo::selected(WidgetType::Checkbox, ui.is_enabled(), selected, label)
    });

    if ui.is_rect_visible(rect) {
        let context = ui.ctx();
        let checked_t = context.animate_bool_with_time(response.id.with("checked"), selected, 0.18);
        let hover_t =
            context.animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.15);
        let press_t = context.animate_bool_with_time(
            response.id.with("press"),
            response.is_pointer_button_down_on(),
            0.10,
        );
        let enabled_alpha = if ui.is_enabled() {
            1.0
        } else {
            ui.visuals().disabled_alpha()
        };
        let painter = ui.painter().with_clip_rect(rect);

        if hover_t > 0.0 {
            painter.rect_filled(
                rect,
                CornerRadius::same(6),
                Color32::from_rgba_unmultiplied(112, 98, 255, (18.0 * hover_t) as u8),
            );
        }

        let icon_center = egui::pos2(rect.left() + 10.0, rect.center().y);
        let icon_rect =
            egui::Rect::from_center_size(icon_center, Vec2::splat(18.0)).shrink(press_t * 0.8);
        let unchecked = Color32::from_rgb(35, 42, 56);
        let fill = mix_color(unchecked, ACCENT, checked_t).gamma_multiply(enabled_alpha);
        let border = mix_color(CARD_BORDER, ACCENT_HOVER, checked_t).gamma_multiply(enabled_alpha);
        painter.rect(
            icon_rect,
            CornerRadius::same(5),
            fill,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );

        if checked_t > 0.0 {
            let p0 = egui::pos2(icon_rect.left() + 4.0, icon_rect.center().y);
            let p1 = egui::pos2(icon_rect.left() + 7.5, icon_rect.bottom() - 4.5);
            let p2 = egui::pos2(icon_rect.right() - 3.5, icon_rect.top() + 4.0);
            let points = if checked_t < 0.48 {
                vec![p0, p0.lerp(p1, checked_t / 0.48)]
            } else {
                vec![p0, p1, p1.lerp(p2, (checked_t - 0.48) / 0.52)]
            };
            painter.add(egui::Shape::line(
                points,
                Stroke::new(2.0, Color32::WHITE.gamma_multiply(enabled_alpha)),
            ));
        }

        painter.text(
            egui::pos2(rect.left() + 30.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::new(14.0, FontFamily::Proportional),
            Color32::WHITE.gamma_multiply(enabled_alpha),
        );
    }

    response
}

fn mode_status_description(state: ModeVisualState) -> String {
    match state {
        ModeVisualState::Desktop => "Desktop mode is active".to_owned(),
        ModeVisualState::Activating(countdown) => {
            format!("Gaming mode activates in {countdown}")
        }
        ModeVisualState::Gaming => "Gaming mode is active".to_owned(),
    }
}

fn mode_led(ui: &mut egui::Ui, state: ModeVisualState) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
    let center = rect.center();
    let painter = ui.painter();

    let powered = !matches!(state, ModeVisualState::Desktop);
    let gaming = matches!(state, ModeVisualState::Gaming);
    let power_t = ui
        .ctx()
        .animate_bool_with_time(response.id.with("power"), powered, 0.30);
    let green_t = ui
        .ctx()
        .animate_bool_with_time(response.id.with("green"), gaming, 0.24);
    let active_color = mix_color(ACTIVATING_ORANGE, GAMING_GREEN, green_t);
    let color = mix_color(LED_OFF, active_color, power_t);
    let pulse = if matches!(state, ModeVisualState::Activating(_)) {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
        let time = ui.input(|input| input.time);
        0.5 + 0.5 * (time * std::f64::consts::TAU * 1.35).sin() as f32
    } else {
        0.35
    };

    painter.circle_filled(
        center,
        7.0 + power_t * (1.0 + pulse),
        color_with_alpha(color, (18.0 + 36.0 * power_t * pulse) as u8),
    );
    painter.circle_filled(
        center,
        5.0 + 0.7 * power_t * pulse,
        color_with_alpha(color, (58.0 + 42.0 * power_t) as u8),
    );
    painter.circle_filled(center, 3.5 + 0.7 * power_t, color);
    if power_t > 0.05 {
        painter.circle_filled(
            center - Vec2::new(1.1, 1.1),
            1.1,
            mix_color(color, Color32::WHITE, 0.68).gamma_multiply(power_t),
        );
    }

    response.on_hover_text(mode_status_description(state));
}

fn animated_text_button(
    ui: &mut egui::Ui,
    label: &str,
    size: Vec2,
    tone: AnimatedButtonTone,
    spinner: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label));

    if ui.is_rect_visible(rect) {
        let context = ui.ctx();
        let hover_t =
            context.animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.16);
        let press_t = context.animate_bool_with_time(
            response.id.with("press"),
            response.is_pointer_button_down_on(),
            0.10,
        );
        let visual_rect = rect.shrink(press_t * 1.35);
        let (base, hover, stroke, font_size) = match tone {
            AnimatedButtonTone::Primary => {
                (ACCENT, ACCENT_HOVER, ACCENT.gamma_multiply(1.25), 16.0)
            }
            AnimatedButtonTone::Secondary => (
                Color32::from_rgb(31, 38, 52),
                Color32::from_rgb(47, 55, 74),
                CARD_BORDER,
                13.0,
            ),
        };
        let mut fill = mix_color(base, hover, hover_t);
        if !ui.is_enabled() {
            fill = fill.gamma_multiply(0.82);
        }
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect(
            visual_rect,
            CornerRadius::same(if matches!(tone, AnimatedButtonTone::Primary) {
                10
            } else {
                8
            }),
            fill,
            Stroke::new(1.0, stroke.gamma_multiply(0.82 + 0.18 * hover_t)),
            StrokeKind::Inside,
        );

        let text_color = if ui.is_enabled() {
            Color32::WHITE
        } else {
            Color32::WHITE.gamma_multiply(0.82)
        };
        let font = FontId::new(font_size, FontFamily::Proportional);
        if spinner {
            let galley = painter.layout_no_wrap(label.to_owned(), font.clone(), text_color);
            let spinner_size = 18.0;
            let gap = 9.0;
            let total_width = spinner_size + gap + galley.size().x;
            let left = visual_rect.center().x - total_width * 0.5;
            let spinner_rect = egui::Rect::from_min_size(
                egui::pos2(left, visual_rect.center().y - spinner_size * 0.5),
                Vec2::splat(spinner_size),
            );
            egui::Spinner::new()
                .size(spinner_size)
                .color(text_color)
                .paint_at(ui, spinner_rect);
            painter.galley(
                egui::pos2(
                    left + spinner_size + gap,
                    visual_rect.center().y - galley.size().y * 0.5,
                ),
                galley,
                text_color,
            );
        } else {
            painter.text(
                visual_rect.center(),
                Align2::CENTER_CENTER,
                label,
                font,
                text_color,
            );
        }
    }

    response
}

fn animated_close_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), "Close"));

    if ui.is_rect_visible(rect) {
        let hover_t =
            ui.ctx()
                .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.16);
        let press_t = ui.ctx().animate_bool_with_time(
            response.id.with("press"),
            response.is_pointer_button_down_on(),
            0.10,
        );
        let visual_rect = rect.shrink(press_t * 1.1);
        ui.painter().rect(
            visual_rect,
            CornerRadius::same(8),
            Color32::from_rgba_unmultiplied(112, 98, 255, (18.0 + 54.0 * hover_t) as u8),
            Stroke::new(
                1.0,
                color_with_alpha(ACCENT_HOVER, (68.0 + 92.0 * hover_t) as u8),
            ),
            StrokeKind::Inside,
        );
        let half = 4.2 - press_t * 0.45;
        let color = mix_color(TEXT_MUTED, Color32::WHITE, hover_t);
        ui.painter().line_segment(
            [
                visual_rect.center() - Vec2::splat(half),
                visual_rect.center() + Vec2::splat(half),
            ],
            Stroke::new(1.7, color),
        );
        ui.painter().line_segment(
            [
                visual_rect.center() + Vec2::new(-half, half),
                visual_rect.center() + Vec2::new(half, -half),
            ],
            Stroke::new(1.7, color),
        );
    }

    response.on_hover_text("Minimize to the system tray")
}

fn window_close_button(context: &egui::Context) -> bool {
    egui::Area::new(egui::Id::new("window-close-button"))
        .anchor(Align2::RIGHT_TOP, Vec2::new(-8.0, 8.0))
        .order(egui::Order::Foreground)
        .movable(false)
        .show(context, animated_close_button)
        .inner
        .clicked()
}

fn animated_selectable_row(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), egui::Sense::click());
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::Button, ui.is_enabled(), selected, label));

    if ui.is_rect_visible(rect) {
        let selected_t =
            ui.ctx()
                .animate_bool_with_time(response.id.with("selected"), selected, 0.18);
        let hover_t =
            ui.ctx()
                .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.15);
        let press_t = ui.ctx().animate_bool_with_time(
            response.id.with("press"),
            response.is_pointer_button_down_on(),
            0.10,
        );
        let enabled_alpha = if ui.is_enabled() {
            1.0
        } else {
            ui.visuals().disabled_alpha()
        };
        let painter = ui.painter().with_clip_rect(rect);
        let base_fill = Color32::from_rgba_unmultiplied(50, 58, 78, (44.0 * hover_t) as u8);
        let selected_fill = Color32::from_rgba_unmultiplied(
            112,
            98,
            255,
            (150.0 * selected_t * enabled_alpha) as u8,
        );
        painter.rect_filled(
            rect.shrink(press_t * 0.7),
            CornerRadius::same(6),
            mix_color(base_fill, selected_fill, selected_t),
        );
        if selected_t > 0.0 {
            let rail = egui::Rect::from_center_size(
                egui::pos2(rect.left() + 2.0, rect.center().y),
                Vec2::new(3.0, (rect.height() - 8.0) * selected_t),
            );
            painter.rect_filled(
                rail,
                CornerRadius::same(2),
                ACCENT_HOVER.gamma_multiply(enabled_alpha),
            );
        }
        let text_color =
            mix_color(TEXT_MUTED, Color32::WHITE, selected_t).gamma_multiply(enabled_alpha);
        painter.text(
            egui::pos2(rect.left() + 9.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::new(12.5, FontFamily::Proportional),
            text_color,
        );
    }

    response
}

fn countdown_digit(started_at: f64, now: f64) -> u8 {
    (3_i32 - (now - started_at).max(0.0).floor() as i32).max(0) as u8
}

fn activating_tray_icon_state(elapsed: f64) -> TrayIconState {
    let phase = (elapsed.max(0.0) / TRAY_BLINK_HALF_PERIOD_SECONDS).floor() as u64;
    if phase.is_multiple_of(2) {
        TrayIconState::ActivatingBright
    } else {
        TrayIconState::ActivatingDim
    }
}

fn tray_mode_menu_presentation(state: ModeVisualState) -> (String, bool) {
    match state {
        ModeVisualState::Desktop => ("Enter Gaming Mode".to_owned(), true),
        ModeVisualState::Activating(countdown) => (format!("Activating in {countdown}…"), false),
        ModeVisualState::Gaming => ("Restore Desktop Mode".to_owned(), true),
    }
}

fn mix_color(left: Color32, right: Color32, amount: f32) -> Color32 {
    egui::lerp(
        egui::Rgba::from(left)..=egui::Rgba::from(right),
        amount.clamp(0.0, 1.0),
    )
    .into()
}

fn color_with_alpha(color: Color32, alpha: u8) -> Color32 {
    let [red, green, blue, _] = color.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(red, green, blue, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_profiles_default_to_the_current_resolution() {
        let mut profile = None;

        initialize_profile(&mut profile, Resolution::new(1920, 1080));

        assert_eq!(profile, Some(Resolution::new(1920, 1080)));
    }

    #[test]
    fn saved_profiles_are_preserved_when_temporarily_unavailable() {
        let current = Resolution::new(1920, 1080);
        let saved = Resolution::new(2560, 1440);
        let mut profile = Some(saved);

        initialize_profile(&mut profile, current);

        assert_eq!(profile, Some(saved));
    }

    #[test]
    fn resolutions_taller_than_native_are_rejected() {
        let native = Some(Resolution::new(2560, 1440));

        assert!(resolution_is_allowed(Resolution::new(3840, 1440), native));
        assert!(resolution_is_allowed(Resolution::new(1920, 1080), native));
        assert!(!resolution_is_allowed(Resolution::new(2560, 1600), native));
    }

    #[test]
    fn disallowed_saved_profile_is_replaced_by_the_allowed_default() {
        let native = Some(Resolution::new(2560, 1440));
        let default = Resolution::new(1920, 1080);
        let mut profile = Some(Resolution::new(2560, 1600));

        replace_disallowed_profile(&mut profile, default, native);

        assert_eq!(profile, Some(default));
    }

    #[test]
    fn gaming_countdown_displays_every_digit_for_one_second() {
        let started_at = 10.0;

        assert_eq!(countdown_digit(started_at, 10.0), 3);
        assert_eq!(countdown_digit(started_at, 10.999), 3);
        assert_eq!(countdown_digit(started_at, 11.0), 2);
        assert_eq!(countdown_digit(started_at, 12.0), 1);
        assert_eq!(countdown_digit(started_at, 13.0), 0);
        assert_eq!(countdown_digit(started_at, 13.999), 0);
    }

    #[test]
    fn tray_mode_menu_tracks_the_current_transition() {
        assert_eq!(
            tray_mode_menu_presentation(ModeVisualState::Desktop),
            ("Enter Gaming Mode".to_owned(), true)
        );
        assert_eq!(
            tray_mode_menu_presentation(ModeVisualState::Activating(2)),
            ("Activating in 2…".to_owned(), false)
        );
        assert_eq!(
            tray_mode_menu_presentation(ModeVisualState::Gaming),
            ("Restore Desktop Mode".to_owned(), true)
        );
    }

    #[test]
    fn activating_tray_led_blinks_twice_per_second() {
        assert_eq!(
            activating_tray_icon_state(0.0),
            TrayIconState::ActivatingBright
        );
        assert_eq!(
            activating_tray_icon_state(0.499),
            TrayIconState::ActivatingBright
        );
        assert_eq!(
            activating_tray_icon_state(0.5),
            TrayIconState::ActivatingDim
        );
        assert_eq!(
            activating_tray_icon_state(0.999),
            TrayIconState::ActivatingDim
        );
        assert_eq!(
            activating_tray_icon_state(1.0),
            TrayIconState::ActivatingBright
        );
    }

    #[test]
    fn late_viewport_hook_overrides_corrupt_saved_geometry() {
        let viewport = egui::ViewportBuilder::default()
            .with_inner_size([160.0, 64.0])
            .with_min_inner_size([64.0, 64.0])
            .with_max_inner_size([160.0, 64.0])
            .with_clamp_size_to_monitor_size(true)
            .with_fullscreen(true)
            .with_maximized(true)
            .with_decorations(true)
            .with_resizable(true)
            .with_maximize_button(true);

        let fixed = fixed_viewport(viewport);
        let size = Some(Vec2::from(WINDOW_SIZE));

        assert_eq!(fixed.inner_size, size);
        assert_eq!(fixed.min_inner_size, size);
        assert_eq!(fixed.max_inner_size, size);
        assert_eq!(fixed.clamp_size_to_monitor_size, Some(false));
        assert_eq!(fixed.fullscreen, Some(false));
        assert_eq!(fixed.maximized, Some(false));
        assert_eq!(fixed.decorations, Some(false));
        assert_eq!(fixed.resizable, Some(false));
        assert_eq!(fixed.maximize_button, Some(false));
    }

    #[test]
    fn iconic_window_size_is_never_accepted_as_fixed() {
        assert!(is_fixed_window_size(Vec2::from(WINDOW_SIZE)));
        assert!(!is_fixed_window_size(Vec2::new(160.0, 64.0)));
    }

    #[test]
    fn fixed_window_height_fits_every_compiled_action_row() {
        assert_eq!(fixed_window_height(0), 475.0);
        assert_eq!(fixed_window_height(1), 509.0);
        assert_eq!(fixed_window_height(2), 543.0);
        assert_eq!(fixed_window_height(3), 577.0);
    }

    #[test]
    fn mode_wordmarks_fit_beside_the_header_and_close_button() {
        for bytes in [
            include_bytes!("../assets/desktop-mode-wordmark.png").as_slice(),
            include_bytes!("../assets/gaming-mode-wordmark.png").as_slice(),
        ] {
            let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
                .unwrap()
                .into_rgba8();
            let (left, top, right, bottom) = alpha_bounds(&image).unwrap();
            let aspect_ratio = (right - left + 1) as f32 / (bottom - top + 1) as f32;

            assert!(aspect_ratio * WORDMARK_DISPLAY_HEIGHT <= 205.0);
        }
    }
}

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use image::{
    DynamicImage, ExtendedColorType, ImageEncoder, Rgba, Rgba32FImage, RgbaImage,
    codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder},
    imageops::FilterType as ResizeFilterType,
};
use tracel_xtask::prelude::*;

mod sound;

const APP_PACKAGE: &str = "win11-borderless-gaming-desktop";
const ACTION_FEATURES: &str = "desktop-icons,desktop-background,minimize-all-windows";
const ICON_MASTER: &str = "crates/win11-borderless-gaming-desktop/assets/icon-master.png";
const ICON_ASSETS: &str = "crates/win11-borderless-gaming-desktop/assets";
const RUNTIME_ASSETS: &str = "crates/win11-borderless-gaming-desktop/assets/runtime";
const DESKTOP_WORDMARK_MASTER: &str =
    "crates/win11-borderless-gaming-desktop/assets/desktop-mode-wordmark.png";
const GAMING_WORDMARK_MASTER: &str =
    "crates/win11-borderless-gaming-desktop/assets/gaming-mode-wordmark.png";
const ICO_SIZES: [u32; 9] = [16, 20, 24, 32, 40, 48, 64, 128, 256];
const APP_ICON_SIZE: u32 = 256;
const TRAY_SIZE: u32 = 32;
const TRAY_SUPERSAMPLING: u32 = 4;
// Keep this in sync with `WORDMARK_TEXTURE_HEIGHT` in `gui.rs`.
const WORDMARK_TEXTURE_HEIGHT: u32 = 96;

const DESKTOP_LED: Rgba<u8> = Rgba([148, 156, 181, 255]);
const GAMING_LED: Rgba<u8> = Rgba([74, 222, 128, 255]);
const ACTIVATING_LED: Rgba<u8> = Rgba([249, 151, 71, 255]);
const ACTIVATING_DIM_LED: Rgba<u8> = Rgba([92, 51, 24, 255]);
const LED_BORDER: Rgba<u8> = Rgba([17, 20, 32, 255]);
const LED_HIGHLIGHT: Rgba<u8> = Rgba([255, 255, 255, 210]);

#[macros::declare_command_args(None, None)]
pub struct RunCmdArgs {
    /// Build and run the one-shot app without the GUI.
    #[arg(long)]
    pub no_gui: bool,
}

#[macros::declare_command_args(None, None)]
pub struct IconsCmdArgs {}

#[macros::declare_command_args(None, None)]
pub struct AssetsCmdArgs {}

#[macros::base_commands]
pub enum Command {
    /// Build the app in release mode with every action, then run it.
    Run(RunCmdArgs),
    /// Generate compact runtime media plus every application and tray icon.
    Assets(AssetsCmdArgs),
    /// Alias for `assets`, retained for compatibility.
    Icons(IconsCmdArgs),
}

fn main() -> anyhow::Result<()> {
    let (args, environment) = init_xtask::<Command>(parse_args::<Command>()?)?;
    match args.command {
        Command::Run(run_args) => handle_run(run_args),
        Command::Assets(_) => handle_assets(),
        Command::Icons(_) => handle_assets(),
        _ => dispatch_base_commands(args, environment),
    }
}

fn handle_run(args: RunCmdArgs) -> anyhow::Result<()> {
    handle_assets()?;

    let status = std::process::Command::new("cargo")
        .args(cargo_run_args(args.no_gui))
        .status()
        .map_err(|error| anyhow::anyhow!("failed to start Cargo: {error}"))?;

    if !status.success() {
        anyhow::bail!("Cargo could not build or run the release app ({status})");
    }

    Ok(())
}

fn cargo_run_args(no_gui: bool) -> Vec<&'static str> {
    let mut args = vec![
        "run",
        "--release",
        "--package",
        APP_PACKAGE,
        "--bin",
        APP_PACKAGE,
    ];

    if no_gui {
        args.extend(["--no-default-features", "--features", ACTION_FEATURES]);
    } else {
        args.push("--all-features");
    }

    args
}

fn handle_icons() -> anyhow::Result<()> {
    let workspace_root = workspace_root()?;
    let master_path = workspace_root.join(ICON_MASTER);
    let assets_path = workspace_root.join(ICON_ASSETS);
    let master = load_master(&master_path)?;

    fs::create_dir_all(&assets_path).map_err(|error| {
        anyhow::anyhow!(
            "failed to create icon assets directory {}: {error}",
            assets_path.display()
        )
    })?;

    let app_png = resize_square(&master, APP_ICON_SIZE);
    write_png(&assets_path.join("app.png"), &app_png)?;
    write_ico(&assets_path.join("app.ico"), &master)?;

    for (name, color, highlight) in [
        ("tray-desktop.png", DESKTOP_LED, Some(LED_HIGHLIGHT)),
        ("tray-gaming.png", GAMING_LED, Some(LED_HIGHLIGHT)),
        ("tray-activating.png", ACTIVATING_LED, Some(LED_HIGHLIGHT)),
        ("tray-activating-dim.png", ACTIVATING_DIM_LED, None),
    ] {
        write_png(
            &assets_path.join(name),
            &tray_variant(&master, color, highlight),
        )?;
    }

    println!("Generated icons from {}:", master_path.display());
    for name in [
        "app.ico",
        "app.png",
        "tray-desktop.png",
        "tray-gaming.png",
        "tray-activating.png",
        "tray-activating-dim.png",
    ] {
        println!("  {}", assets_path.join(name).display());
    }

    Ok(())
}

fn handle_assets() -> anyhow::Result<()> {
    handle_icons()?;

    let workspace_root = workspace_root()?;
    let runtime_path = workspace_root.join(RUNTIME_ASSETS);
    let icon_master = load_master(&workspace_root.join(ICON_MASTER))?;

    fs::create_dir_all(&runtime_path).map_err(|error| {
        anyhow::anyhow!(
            "failed to create runtime assets directory {}: {error}",
            runtime_path.display()
        )
    })?;

    write_png(
        &runtime_path.join("app.png"),
        &resize_square(&icon_master, APP_ICON_SIZE),
    )?;

    for (name, color, highlight) in [
        ("tray-desktop.png", DESKTOP_LED, Some(LED_HIGHLIGHT)),
        ("tray-gaming.png", GAMING_LED, Some(LED_HIGHLIGHT)),
        ("tray-activating.png", ACTIVATING_LED, Some(LED_HIGHLIGHT)),
        ("tray-activating-dim.png", ACTIVATING_DIM_LED, None),
    ] {
        write_png(
            &runtime_path.join(name),
            &tray_variant(&icon_master, color, highlight),
        )?;
    }

    for (master, output) in [
        (DESKTOP_WORDMARK_MASTER, "desktop-mode-wordmark.png"),
        (GAMING_WORDMARK_MASTER, "gaming-mode-wordmark.png"),
    ] {
        let master_path = workspace_root.join(master);
        let source = load_rgba(&master_path)?;
        let runtime_wordmark = prepare_wordmark(&source).ok_or_else(|| {
            anyhow::anyhow!(
                "wordmark master {} has no visible pixels",
                master_path.display()
            )
        })?;
        write_png(&runtime_path.join(output), &runtime_wordmark)?;
    }

    for cue in sound::Cue::ALL {
        let output = runtime_path.join(cue.file_name());
        fs::write(&output, sound::wav_bytes(cue)).map_err(|error| {
            anyhow::anyhow!("failed to write sound cue {}: {error}", output.display())
        })?;
    }

    println!("Generated compact runtime assets:");
    for name in [
        "app.png",
        "desktop-mode-wordmark.png",
        "gaming-mode-wordmark.png",
        "tray-desktop.png",
        "tray-gaming.png",
        "tray-activating.png",
        "tray-activating-dim.png",
        "countdown.wav",
        "gaming-enter.wav",
        "gaming-leave.wav",
    ] {
        println!("  {}", runtime_path.join(name).display());
    }

    Ok(())
}

fn workspace_root() -> anyhow::Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow::anyhow!("xtask manifest directory has no workspace parent"))
}

fn load_master(path: &Path) -> anyhow::Result<RgbaImage> {
    let image = load_rgba(path)?;

    if image.width() != image.height() {
        anyhow::bail!(
            "icon master must be square, but {} is {}x{}",
            path.display(),
            image.width(),
            image.height()
        );
    }
    if image.width() < 512 {
        anyhow::bail!(
            "icon master must be at least 512x512, but {} is {}x{}",
            path.display(),
            image.width(),
            image.height()
        );
    }
    if !image.pixels().any(|pixel| pixel[3] < u8::MAX) {
        anyhow::bail!(
            "icon master {} has no transparent pixels; genuine alpha is required",
            path.display()
        );
    }

    Ok(image)
}

fn load_rgba(path: &Path) -> anyhow::Result<RgbaImage> {
    image::open(path)
        .map_err(|error| anyhow::anyhow!("failed to load artwork {}: {error}", path.display()))
        .map(DynamicImage::into_rgba8)
}

fn resize_square(source: &RgbaImage, size: u32) -> RgbaImage {
    DynamicImage::ImageRgba8(source.clone())
        .resize_exact(size, size, ResizeFilterType::Lanczos3)
        .into_rgba8()
}

fn prepare_wordmark(source: &RgbaImage) -> Option<RgbaImage> {
    let (left, top, right, bottom) = alpha_bounds(source)?;
    let cropped = image::imageops::crop_imm(
        source,
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

    Some(resize_wordmark_antialiased(
        &cropped,
        target_width,
        WORDMARK_TEXTURE_HEIGHT,
    ))
}

fn resize_wordmark_antialiased(source: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    // Resample in linear light with premultiplied alpha. Straight-alpha sRGB
    // resizing lets arbitrary RGB values from transparent pixels bleed into
    // edges, which shows up as dark fringes and jagged letter contours.
    let premultiplied = Rgba32FImage::from_fn(source.width(), source.height(), |x, y| {
        let pixel = source.get_pixel(x, y);
        // The source masters contain near-transparent export noise. Treat it
        // consistently with `alpha_bounds` so isolated speckles do not become
        // visible during minification.
        let alpha = if pixel[3] <= 8 {
            0.0
        } else {
            f32::from(pixel[3]) / 255.0
        };
        Rgba([
            srgb_to_linear(pixel[0]) * alpha,
            srgb_to_linear(pixel[1]) * alpha,
            srgb_to_linear(pixel[2]) * alpha,
            alpha,
        ])
    });
    let resized =
        image::imageops::resize(&premultiplied, width, height, ResizeFilterType::Lanczos3);

    RgbaImage::from_fn(width, height, |x, y| {
        let pixel = resized.get_pixel(x, y);
        let alpha = pixel[3].clamp(0.0, 1.0);
        let alpha_u8 = (alpha * 255.0).round() as u8;
        if alpha_u8 == 0 {
            return Rgba([0, 0, 0, 0]);
        }

        Rgba([
            linear_to_srgb(pixel[0].clamp(0.0, alpha) / alpha),
            linear_to_srgb(pixel[1].clamp(0.0, alpha) / alpha),
            linear_to_srgb(pixel[2].clamp(0.0, alpha) / alpha),
            alpha_u8,
        ])
    })
}

fn srgb_to_linear(value: u8) -> f32 {
    let value = f32::from(value) / 255.0;
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as u8
}

fn alpha_bounds(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (width, height) = image.dimensions();
    let mut left = width;
    let mut top = height;
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;

    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] <= 8 {
            continue;
        }
        found = true;
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
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

fn tray_variant(
    master: &RgbaImage,
    led_color: Rgba<u8>,
    highlight_color: Option<Rgba<u8>>,
) -> RgbaImage {
    let canvas_size = TRAY_SIZE * TRAY_SUPERSAMPLING;
    let mut canvas = resize_square(master, canvas_size);
    let center = (canvas_size as i32 * 25 / 32, canvas_size as i32 * 25 / 32);
    let border_radius = canvas_size as i32 * 7 / 32;
    let led_radius = canvas_size as i32 * 5 / 32;

    draw_circle(&mut canvas, center, border_radius, LED_BORDER);
    draw_circle(&mut canvas, center, led_radius, led_color);
    if let Some(highlight_color) = highlight_color {
        draw_circle(
            &mut canvas,
            (center.0 - led_radius / 3, center.1 - led_radius / 3),
            (led_radius / 4).max(1),
            highlight_color,
        );
    }

    resize_square(&canvas, TRAY_SIZE)
}

fn draw_circle(image: &mut RgbaImage, center: (i32, i32), radius: i32, color: Rgba<u8>) {
    let min_x = (center.0 - radius).max(0) as u32;
    let max_x = (center.0 + radius).min(image.width() as i32 - 1) as u32;
    let min_y = (center.1 - radius).max(0) as u32;
    let max_y = (center.1 + radius).min(image.height() as i32 - 1) as u32;
    let radius_squared = radius * radius;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as i32 - center.0;
            let dy = y as i32 - center.1;
            if dx * dx + dy * dy <= radius_squared {
                alpha_composite(image.get_pixel_mut(x, y), color);
            }
        }
    }
}

fn alpha_composite(destination: &mut Rgba<u8>, source: Rgba<u8>) {
    let source_alpha = source[3] as f32 / 255.0;
    let destination_alpha = destination[3] as f32 / 255.0;
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);

    if output_alpha <= f32::EPSILON {
        *destination = Rgba([0, 0, 0, 0]);
        return;
    }

    for channel in 0..3 {
        let value = (source[channel] as f32 * source_alpha
            + destination[channel] as f32 * destination_alpha * (1.0 - source_alpha))
            / output_alpha;
        destination[channel] = value.round().clamp(0.0, 255.0) as u8;
    }
    destination[3] = (output_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn write_png(path: &Path, image: &RgbaImage) -> anyhow::Result<()> {
    let file = File::create(path)
        .map_err(|error| anyhow::anyhow!("failed to create {}: {error}", path.display()))?;
    PngEncoder::new_with_quality(
        BufWriter::new(file),
        CompressionType::Best,
        PngFilterType::Adaptive,
    )
    .write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        ExtendedColorType::Rgba8,
    )
    .map_err(|error| anyhow::anyhow!("failed to encode {}: {error}", path.display()))
}

fn write_ico(path: &Path, master: &RgbaImage) -> anyhow::Result<()> {
    let bytes = encode_ico(master)?;
    fs::write(path, bytes)
        .map_err(|error| anyhow::anyhow!("failed to write {}: {error}", path.display()))
}

fn encode_ico(master: &RgbaImage) -> anyhow::Result<Vec<u8>> {
    let png_images = ICO_SIZES
        .iter()
        .map(|size| encode_png(&resize_square(master, *size)))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let header_size = 6 + png_images.len() * 16;
    let capacity = header_size + png_images.iter().map(Vec::len).sum::<usize>();
    let mut ico = Vec::with_capacity(capacity);

    ico.write_all(&0_u16.to_le_bytes())?;
    ico.write_all(&1_u16.to_le_bytes())?;
    ico.write_all(&(png_images.len() as u16).to_le_bytes())?;

    let mut image_offset = header_size as u32;
    for (size, png) in ICO_SIZES.iter().zip(&png_images) {
        ico.write_all(&[if *size == 256 { 0 } else { *size as u8 }])?;
        ico.write_all(&[if *size == 256 { 0 } else { *size as u8 }])?;
        ico.write_all(&[0, 0])?;
        ico.write_all(&1_u16.to_le_bytes())?;
        ico.write_all(&32_u16.to_le_bytes())?;
        ico.write_all(&(png.len() as u32).to_le_bytes())?;
        ico.write_all(&image_offset.to_le_bytes())?;
        image_offset += png.len() as u32;
    }

    for png in png_images {
        ico.write_all(&png)?;
    }

    Ok(ico)
}

fn encode_png(image: &RgbaImage) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Best, PngFilterType::Adaptive)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_run_uses_release_and_every_feature() {
        assert_eq!(
            cargo_run_args(false),
            [
                "run",
                "--release",
                "--package",
                APP_PACKAGE,
                "--bin",
                APP_PACKAGE,
                "--all-features",
            ]
        );
    }

    #[test]
    fn no_gui_run_uses_release_and_every_action_feature() {
        assert_eq!(
            cargo_run_args(true),
            [
                "run",
                "--release",
                "--package",
                APP_PACKAGE,
                "--bin",
                APP_PACKAGE,
                "--no-default-features",
                "--features",
                ACTION_FEATURES,
            ]
        );
    }

    #[test]
    fn generated_ico_contains_every_configured_size() {
        let mut master = RgbaImage::new(512, 512);
        draw_circle(&mut master, (256, 256), 192, GAMING_LED);

        let ico = encode_ico(&master).expect("ICO encoding should succeed");

        assert_eq!(&ico[0..6], &[0, 0, 1, 0, ICO_SIZES.len() as u8, 0]);
        for (index, size) in ICO_SIZES.iter().enumerate() {
            let entry = 6 + index * 16;
            let encoded_size = if *size == 256 { 0 } else { *size as u8 };
            assert_eq!(ico[entry], encoded_size);
            assert_eq!(ico[entry + 1], encoded_size);
        }
    }

    #[test]
    fn runtime_wordmark_is_cropped_and_preserves_aspect_ratio() {
        let mut master = RgbaImage::new(400, 200);
        for y in 50..150 {
            for x in 50..350 {
                master.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }

        let runtime = prepare_wordmark(&master).expect("wordmark should contain visible pixels");

        assert_eq!(runtime.dimensions(), (267, WORDMARK_TEXTURE_HEIGHT));
        assert_eq!(runtime.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn transparent_wordmark_has_no_runtime_asset() {
        assert!(prepare_wordmark(&RgbaImage::new(400, 200)).is_none());
    }

    #[test]
    fn wordmark_resampling_antialiases_without_transparent_color_bleed() {
        let mut source = RgbaImage::from_pixel(32, 32, Rgba([0, 0, 255, 0]));
        for y in 6..26 {
            for x in 6..26 {
                source.put_pixel(x, y, Rgba([255, 32, 16, 255]));
            }
        }

        let resized = resize_wordmark_antialiased(&source, 13, 13);
        let antialiased_edges = resized
            .pixels()
            .filter(|pixel| (1..=254).contains(&pixel[3]))
            .collect::<Vec<_>>();

        assert!(!antialiased_edges.is_empty());
        assert!(antialiased_edges.iter().all(|pixel| pixel[0] > pixel[2]));
        assert!(
            resized
                .pixels()
                .filter(|pixel| pixel[3] == 0)
                .all(|pixel| pixel.0 == [0, 0, 0, 0])
        );
    }

    #[test]
    fn committed_runtime_assets_match_their_masters() {
        let root = workspace_root().expect("workspace root should exist");
        let runtime_path = root.join(RUNTIME_ASSETS);
        let icon_master = load_master(&root.join(ICON_MASTER)).expect("icon master should load");

        assert_image_eq(
            &resize_square(&icon_master, APP_ICON_SIZE),
            &load_rgba(&runtime_path.join("app.png")).expect("runtime app icon should load"),
        );

        for (name, color, highlight) in [
            ("tray-desktop.png", DESKTOP_LED, Some(LED_HIGHLIGHT)),
            ("tray-gaming.png", GAMING_LED, Some(LED_HIGHLIGHT)),
            ("tray-activating.png", ACTIVATING_LED, Some(LED_HIGHLIGHT)),
            ("tray-activating-dim.png", ACTIVATING_DIM_LED, None),
        ] {
            assert_image_eq(
                &tray_variant(&icon_master, color, highlight),
                &load_rgba(&runtime_path.join(name)).expect("runtime tray icon should load"),
            );
        }

        for (master, output) in [
            (DESKTOP_WORDMARK_MASTER, "desktop-mode-wordmark.png"),
            (GAMING_WORDMARK_MASTER, "gaming-mode-wordmark.png"),
        ] {
            let source = load_rgba(&root.join(master)).expect("wordmark master should load");
            let expected = prepare_wordmark(&source).expect("wordmark should be visible");
            let actual =
                load_rgba(&runtime_path.join(output)).expect("runtime wordmark should load");
            assert_image_eq(&expected, &actual);
        }

        for cue in sound::Cue::ALL {
            let actual = fs::read(runtime_path.join(cue.file_name()))
                .expect("runtime sound cue should load");
            assert_eq!(actual, sound::wav_bytes(cue));
        }
    }

    fn assert_image_eq(expected: &RgbaImage, actual: &RgbaImage) {
        assert_eq!(actual.dimensions(), expected.dimensions());
        assert_eq!(actual.as_raw(), expected.as_raw());
    }

    #[test]
    fn tray_variant_keeps_transparency_and_adds_status_color() {
        let master = RgbaImage::new(512, 512);
        let tray = tray_variant(&master, ACTIVATING_LED, Some(LED_HIGHLIGHT));

        assert_eq!(tray.dimensions(), (TRAY_SIZE, TRAY_SIZE));
        assert_eq!(tray.get_pixel(0, 0)[3], 0);
        let status = tray.get_pixel(25, 25);
        assert!(status[0] > status[1]);
        assert!(status[1] > status[2]);
        assert!(status[3] > 240);
    }

    #[test]
    fn dim_activating_variant_is_darker_and_has_no_white_highlight() {
        let master = RgbaImage::new(512, 512);
        let bright = tray_variant(&master, ACTIVATING_LED, Some(LED_HIGHLIGHT));
        let dim = tray_variant(&master, ACTIVATING_DIM_LED, None);
        let brightness =
            |pixel: &Rgba<u8>| u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]);

        let bright_status = bright.get_pixel(25, 25);
        let dim_status = dim.get_pixel(25, 25);
        assert!(brightness(dim_status) < brightness(bright_status));
        assert!(dim_status[0] > dim_status[1]);
        assert!(dim_status[1] > dim_status[2]);

        // The highlight is painted up and left of the LED center. Keeping it
        // out of the dim frame makes a blink look like one orange LED changing
        // intensity instead of a white pixel flashing over a dark badge.
        let highlighted_pixel = bright.get_pixel(23, 23);
        let unhighlighted_pixel = dim.get_pixel(23, 23);
        assert!(brightness(unhighlighted_pixel) < brightness(highlighted_pixel));
    }
}

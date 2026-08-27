use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use image::{
    DynamicImage, ExtendedColorType, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder,
    imageops::FilterType,
};
use tracel_xtask::prelude::*;

const APP_PACKAGE: &str = "win11-borderless-gaming-desktop";
const ACTION_FEATURES: &str = "desktop-icons,desktop-background,minimize-all-windows";
const ICON_MASTER: &str = "crates/win11-borderless-gaming-desktop/assets/icon-master.png";
const ICON_ASSETS: &str = "crates/win11-borderless-gaming-desktop/assets";
const ICO_SIZES: [u32; 9] = [16, 20, 24, 32, 40, 48, 64, 128, 256];
const TRAY_SIZE: u32 = 32;
const TRAY_SUPERSAMPLING: u32 = 4;

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

#[macros::base_commands]
pub enum Command {
    /// Build the app in release mode with every action, then run it.
    Run(RunCmdArgs),
    /// Derive every application and tray icon from the transparent master artwork.
    Icons(IconsCmdArgs),
}

fn main() -> anyhow::Result<()> {
    let (args, environment) = init_xtask::<Command>(parse_args::<Command>()?)?;
    match args.command {
        Command::Run(run_args) => handle_run(run_args),
        Command::Icons(_) => handle_icons(),
        _ => dispatch_base_commands(args, environment),
    }
}

fn handle_run(args: RunCmdArgs) -> anyhow::Result<()> {
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

    let app_png = resize_square(&master, 256);
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

fn workspace_root() -> anyhow::Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow::anyhow!("xtask manifest directory has no workspace parent"))
}

fn load_master(path: &Path) -> anyhow::Result<RgbaImage> {
    let image = image::open(path)
        .map_err(|error| anyhow::anyhow!("failed to load icon master {}: {error}", path.display()))?
        .into_rgba8();

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

fn resize_square(source: &RgbaImage, size: u32) -> RgbaImage {
    DynamicImage::ImageRgba8(source.clone())
        .resize_exact(size, size, FilterType::Lanczos3)
        .into_rgba8()
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
    PngEncoder::new(BufWriter::new(file))
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
    PngEncoder::new(&mut bytes).write_image(
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

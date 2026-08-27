# Artwork assets

`icon-master.png`, `desktop-mode-wordmark.png`, and
`gaming-mode-wordmark.png` are the high-resolution source artwork. Regenerate
all release-ready artwork from the repository root with:

```sh
cargo xtask assets
```

The command produces the multi-resolution `app.ico`, the preview `app.png`,
the top-level tray status variants, and every file under `runtime/`. The GUI
embeds only the compact, lossless files under `runtime/`; the high-resolution
masters are never copied into the executable. Wordmarks are cropped and
Lanczos-resized to the same 96 px textures the app previously created at
startup, so this moves work and bytes out of the release without changing the
rendered artwork.

Do not edit generated files by hand. `cargo xtask icons` remains as a
backward-compatible alias for the full asset pipeline.

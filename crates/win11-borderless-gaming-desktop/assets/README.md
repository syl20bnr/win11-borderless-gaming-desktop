# Runtime assets

`icon-master.png`, `desktop-mode-wordmark.png`, and
`gaming-mode-wordmark.png` are the high-resolution source artwork. Regenerate
all release-ready artwork from the repository root with:

```sh
cargo xtask assets
```

The command produces the multi-resolution `app.ico`, the preview `app.png`,
the top-level tray status variants, and every file under `runtime/`. The app
embeds the compact artwork and generated cues under `runtime/`. The
high-resolution artwork masters are never copied into the executable. Wordmarks
are cropped and Lanczos-resized
to 96 px textures in linear light with premultiplied alpha, preventing hidden
transparent colors from bleeding into their edges. The app adds linear mipmaps
when minifying those textures so the Desktop and Gaming labels stay smooth.

The countdown, Gaming Mode activation, and Gaming Mode restoration WAV files
are generated from the deterministic integer synthesizer in
`xtask/src/sound.rs`. Together they use about 84 KiB of 48 kHz, 16-bit mono PCM,
need no bundled decoder, and have no external recording or licensing source.
Short-effect loudness is
normalized over a 100 ms window to -23 dB with a -12 dBFS peak ceiling. Smooth
C2-continuous attack and release envelopes, explicit silent padding, a
high-precision fixed-point oscillator, and non-interrupting playback keep the
output free of clicks, clipping, aliasing, and synthesis noise. Every voice is
phase-continuous for its full cue; there are no hard-spliced secondary accents.
The button-click animation also provides a short zero-PCM warm-up window. The
first countdown pulse is queued directly behind it on one persistent native
Windows output handle, preventing a cold device start from contaminating the
audible cue without keeping a looping silent stream alive.

PCM is deliberate for these sub-second UI cues: it is lossless and starts
immediately through the native Windows sound API. A frame-based lossy codec
would save only a few KiB after playback support while adding codec delay and
transient artifacts.

Do not edit generated files by hand. `cargo xtask icons` remains as a
backward-compatible alias for the full asset pipeline.

# win11-taskbar-autohide

Small Rust utility that toggles Windows taskbar auto-hide using the official `SHAppBarMessage` Win32 API.

## What it does

On each run, it:

1. Reads the current taskbar state (`ABM_GETSTATE`).
2. Toggles `ABS_AUTOHIDE` on/off.
3. Writes the updated state (`ABM_SETSTATE`).

## Build

```bash
cargo build --release
```

The binary will be at `target/release/win11-taskbar-autohide.exe` (when built on Windows).

## Usage

Run the executable to toggle auto-hide.

You can pin the `.exe` to the taskbar for one-click toggling.

## Notes

- Uses the documented appbar API; no Explorer restart required.
- Historically, some Windows 11 builds had appbar-related regressions. Current Windows 11 versions generally behave as expected.

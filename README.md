# win11-borderless-gaming-desktop

A tiny utility to make Windows cleaner for borderless gaming.

## Demo

![Demo](https://raw.githubusercontent.com/syl20bnr/win11-borderless-gaming-desktop/refs/heads/main/crates/win11-borderless-gaming-desktop/assets/demo.gif)

---

## Install

### Portable executable (recommended)

Download the latest version from GitHub releases:

👉 https://github.com/syl20bnr/win11-borderless-gaming-desktop/releases/latest

Place the executable anywhere on your disk, then pin it to the taskbar for quick access.

---

### With Cargo (for Rust developers)

```sh
cargo install win11-borderless-gaming-desktop
```

The executable will be installed in Cargo’s `bin` directory. The default crates.io build launches the GUI with every desktop action available.

---

## What it does

Use the app to switch a **borderless gaming mode**:

- enables taskbar auto-hide
- hides desktop icons *(optional feature)*
- replaces desktop background with solid black *(optional feature)*
- minimizes all open windows *(optional feature)*

---

## Usage

The app has two build modes.

### GUI mode (default)

The default build launches a persistent control panel. Opening the GUI does not toggle Gaming mode or change any desktop setting—the main toggle stays inside the window.

```sh
cargo install win11-borderless-gaming-desktop
```

![Borderless Gaming Desktop GUI](https://raw.githubusercontent.com/syl20bnr/win11-borderless-gaming-desktop/refs/heads/main/crates/win11-borderless-gaming-desktop/assets/gui.png)

The GUI includes:

- a live Desktop/Gaming mode indicator with a glowing status LED
- animated controls and a 3 → 2 → 1 → 0 activation countdown with an orange pulsing LED
- a disabled purple Gaming mode button and spinner while activation is pending
- persistent controls for each optional behavior compiled into the executable
- separate persistent Desktop and Gaming resolution profiles for the primary monitor
- resolution changes only when switching modes, with the monitor's preferred mode labeled `(native)` and taller modes filtered out
- regular Windows minimization; double-click the tray icon to restore the window
- Desktop and Gaming colors mirrored by the tray icon, with a blinking orange LED during activation
- **Open**, **Enter Gaming Mode / Restore Desktop Mode**, and **Quit** actions when you right-click the tray icon

The window has a polished, locked layout and never saves or restores native window geometry. Minimizing uses the regular Windows taskbar behavior. The custom close button explains its behavior once, then hides the app to the system tray without leaving a taskbar button. Use **Quit** from the tray menu to exit. GUI choices are saved automatically and restored on the next launch.

### One-shot mode

Compile without the `gui` feature to preserve the original one-shot behavior. Each launch immediately switches between:

- 🖥️ Normal desktop mode
- 🎮 Borderless gaming mode

The app applies the actions compiled into the executable, then exits. Pin it to the taskbar for instant switching.

```bash
cargo build --release --no-default-features --features desktop-icons,desktop-background,minimize-all-windows
```

---

## Build

Default GUI build with all desktop actions enabled:

```bash
cargo build --release
```

For development, the xtask runner always builds in release mode with every desktop action enabled, then launches the app:

```bash
cargo xtask run
```

Pass `--no-gui` to build and execute the original one-shot mode with every desktop action but without the `gui` feature:

```bash
cargo xtask run --no-gui
```

Regenerate the Windows application icon and all Desktop, Activating, and Gaming tray variants from the transparent master artwork:

```bash
cargo xtask icons
```

---

## Features

Enabled by default:

- `gui` → open the persistent egui control panel, resolution profiles, and system tray
- `desktop-icons` → toggle desktop icons
- `desktop-background` → toggle desktop background
- `minimize-all-windows` → minimize all open windows when enabling gaming mode

---

### Examples

One-shot mode with all desktop actions:

```bash
cargo build --release --no-default-features --features desktop-icons,desktop-background,minimize-all-windows
```

One-shot mode with only background handling:

```bash
cargo build --release --no-default-features --features desktop-background
```

One-shot mode with only desktop icons:

```bash
cargo build --release --no-default-features --features desktop-icons
```

One-shot mode with only minimize-all-windows:

```bash
cargo build --release --no-default-features --features minimize-all-windows
```

Minimal one-shot build (taskbar only):

```bash
cargo build --release --no-default-features
```

GUI with only selected actions compiled in:

```bash
cargo build --release --no-default-features --features gui,desktop-icons,desktop-background
```

GUI with taskbar handling only:

```bash
cargo build --release --no-default-features --features gui
```

---

## License

GNU GPL-3.0

# win11-borderless-gaming-desktop

A tiny utility to make Windows cleaner for borderless gaming.

## Demo

![Demo](https://raw.githubusercontent.com/syl20bnr/win11-borderless-gaming-desktop/refs/heads/main/crates/win11-borderless-gaming-desktop/assets/demo.gif)

---

## Install

### Portable executable (recommended)

Download the latest version from GitHub releases:

👉 https://github.com/syl20bnr/win11-borderless-gaming-desktop/releases/latest

Place the executable anywhere on your disk, then drag & drop it to the taskbar for one-click toggle.

---

### With Cargo (for Rust developers)

```sh
cargo install win11-borderless-gaming-desktop
```

The executable will be installed in Cargo’s `bin` directory.
Pin it to your taskbar for quick access.

---

## What it does

Run the app to toggle a **borderless gaming mode**:

- enables taskbar auto-hide
- hides desktop icons *(optional feature)*
- replaces desktop background with solid black *(optional feature)*
- minimizes all open windows *(optional feature)*

---

## Usage

Run the executable to switch between:

- 🖥️ Normal desktop mode
- 🎮 Borderless gaming mode

Tip: pin it to the taskbar for instant switching.

---

## Build

Default build (all features enabled):

```bash
cargo build --release
```

---

## Features

Enabled by default:

- `desktop-icons` → toggle desktop icons
- `desktop-background` → toggle desktop background
- `minimize-all-windows` → minimize all open windows when enabling gaming mode

---

### Examples

Only background handling:

```bash
cargo build --release --no-default-features --features desktop-background
```

Only desktop icons:

```bash
cargo build --release --no-default-features --features desktop-icons
```

Only minimize all windows:

```bash
cargo build --release --no-default-features --features minimize-all-windows
```

Minimal build (taskbar only):

```bash
cargo build --release --no-default-features
```

---

## License

GNU GPL-3.0

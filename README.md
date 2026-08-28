<div align="center">
  <img
    src="https://raw.githubusercontent.com/syl20bnr/win11-borderless-gaming-desktop/refs/heads/main/crates/win11-borderless-gaming-desktop/assets/readme-logo-master.png"
    width="680"
    alt="Borderless Gaming Desktop logo"
  >

  <h1>Borderless Gaming Desktop</h1>

  <p><strong>The perfect gaming XP in one click.</strong></p>
  <p>Built for Ultrawide monitors: work wide, then game focused. 🎮</p>

  <p>
    <a href="https://github.com/syl20bnr/win11-borderless-gaming-desktop/releases/latest">
      <img src="https://img.shields.io/badge/Download_for_Windows-7062FF?style=for-the-badge&logo=windows11&logoColor=white" alt="Download for Windows">
    </a>
    <img src="https://img.shields.io/badge/Portable-no_installer-4ADE80?style=for-the-badge" alt="Portable application">
  </p>

  <p>
    <a href="https://github.com/syl20bnr/win11-borderless-gaming-desktop/releases/latest">
      <img src="https://img.shields.io/github/v/release/syl20bnr/win11-borderless-gaming-desktop?style=flat-square&color=7062FF" alt="Latest release">
    </a>
    <img src="https://img.shields.io/badge/Windows-11-0078D4?style=flat-square&logo=windows11&logoColor=white" alt="Windows 11">
    <a href="https://github.com/syl20bnr/win11-borderless-gaming-desktop/blob/main/LICENSE">
      <img src="https://img.shields.io/badge/license-GPL--3.0-4ADE80?style=flat-square" alt="GPL-3.0 license">
    </a>
  </p>
</div>

---

Borderless Gaming Desktop is made for Ultrawide monitors. It gives your Windows
11 battlestation two loadouts: a comfortable **Desktop Mode** and a
distraction-free **Gaming Mode**, each with its own resolution profile. Keep the
native canvas for everyday work, then switch resolution and clear distractions
for gaming in one click. The normal v2 experience is the portable GUI, no
installer, no terminal, and no surprise desktop changes just because you
launched it. The GUI can also start with Windows, keep its automatic login
launch minimized, and tune whole-window transparency.

### Ready, player one?

1. [Download the latest portable executable from GitHub Releases](https://github.com/syl20bnr/win11-borderless-gaming-desktop/releases/latest).
2. Put the `.exe` wherever you like and double-click it. That is the whole installation.
3. Pick your Desktop and Gaming resolution profiles, choose your Gaming Mode actions and application behavior, then hit the big purple button.

<p align="center">
  <img
    src="https://raw.githubusercontent.com/syl20bnr/win11-borderless-gaming-desktop/refs/heads/main/crates/win11-borderless-gaming-desktop/assets/gui.png"
    width="666"
    alt="Borderless Gaming Desktop in Gaming Mode with action, resolution, startup, and transparency controls"
  >
</p>

### Build your Gaming Mode loadout

The portable release is ready to:

- hide the Windows taskbar with auto-hide
- hide desktop icons
- swap the desktop wallpaper for solid black
- minimize open windows when entering Gaming Mode
- switch your primary Ultrawide monitor to your chosen Gaming resolution

Every optional desktop action has its own checkbox. Sound effects have a separate
**Enable sounds** checkbox and are enabled by default. Selecting a
resolution never changes it immediately, the Desktop or Gaming profile is applied
only when you switch modes. On an Ultrawide, you can keep the native canvas for
Desktop Mode and choose a narrower, game-friendly profile without changing
resolutions by hand. Your monitor's preferred resolution is labeled `(native)`.
While Gaming Mode is active, its desktop-action checkboxes are locked until you
restore Desktop Mode; **Enable sounds** remains adjustable.

### Tune the application itself

The **Application behavior** card controls how the utility starts and looks:

- **Start at login** adds a per-user Windows login entry, with no administrator
  access required.
- **Start minimized** keeps that automatic login launch out of the way; normal
  manual launches still open the window.
- **Window transparency** adjusts the whole panel live from fully opaque to an
  80% safety limit, so it can never become completely invisible.

The current build version is shown unobtrusively in the bottom-right corner.

### Two modes. One purple button.

|                | Desktop Mode 🖥️               | Gaming Mode 🎮                |
|----------------|------------------------------|-------------------------------|
| **Taskbar**    | Normal                       | Auto-hide                     |
| **Desktop**    | Icons and wallpaper restored | Your selected cleanup actions |
| **Resolution** | Desktop profile              | Gaming profile                |
| **Status LED** | Cool gray                    | Bright green                  |

Press **Enter gaming mode** and the button finishes its click animation before
starting the spinner and **3 → 2 → 1** countdown. The wordmark stays **Desktop
mode** throughout the countdown. When sound effects are enabled, each number
gets a soft sci-fi pulse; the app silently primes its native audio output during
the click animation so the first pulse starts cleanly. When **1** finishes, the
spinner disappears, the wordmark switches to **Gaming mode**, and the button
shows **Activating...** while Windows applies the mode changes. The LED pulses
orange while the tray icon blinks, then turns green as a short power-up cue
confirms that the battlestation is ready.

Press **Restore desktop mode** when the match is over. Its click animation
finishes before the wordmark switches to **Desktop mode**, the LED turns orange
and pulses, and the spinner-free button changes to **Restoring...** while Windows
restores the taskbar, selected desktop actions, and Desktop resolution. When
sounds are enabled, a compact power-down cue confirms the switch. Windows
minimized on entry stay minimized, the app will not unexpectedly reopen a pile
of windows over your post-game screen.

### Your loadout remembers you

Action checkboxes, the sound-effects preference, both resolution profiles, the
minimized-start preference, and window transparency are saved automatically.
**Start at login** reflects the real Windows login entry instead of keeping a
separate stale flag. Close the app, reboot, come back tomorrow, your choices are
still equipped.

The mode itself follows Windows taskbar auto-hide. If auto-hide is already enabled when the app starts, the app correctly opens in Gaming Mode instead of keeping a separate, stale mode flag.

### Living in the system tray

- The custom close button and `Alt+F4` hide the app in the system tray without leaving it in the taskbar. The first close explains this once.
- Double-click the tray icon to reopen the window.
- Right-click it to **Open**, **Enter Gaming Mode / Restore Desktop Mode**, or **Quit**.
- The tray LED mirrors the app: gray for Desktop, blinking orange during the countdown and while either mode is being applied, and green for Gaming.
- A regular Windows minimize remains a regular taskbar minimize.

> [!TIP]
> **Quit** means quit, it does not silently change your mode. If you are still in Gaming Mode, restore Desktop Mode first when you want the everyday desktop back.

## Advanced Users and Developers

The GUI is the v2 default. The original v1.1 launch-to-toggle workflow still exists as a deliberate non-GUI build for scripts, custom shortcuts, and power users, but it is no longer the recommended way to use the application.

### Build or install the default GUI

Requirements: **Windows 11** and **Rust 1.94 or newer**.

Install the published crates.io build with all default features:

```powershell
cargo install win11-borderless-gaming-desktop
```

Or build this repository:

```powershell
cargo build --release --package win11-borderless-gaming-desktop
```

The executable is written to:

```text
target/release/win11-borderless-gaming-desktop.exe
```

A normal manual launch opens the GUI and does not toggle anything on startup.
When **Start at login** is enabled, Windows launches the same portable executable
for the current user; **Start minimized** applies only to that automatic launch.

### Cargo features

| Feature                | Default | What it adds                                                                                        |
|------------------------|:-------:|-----------------------------------------------------------------------------------------------------|
| `gui`                  | ✅      | Persistent egui control panel, system tray, startup/transparency settings, and resolution profiles  |
| `sound`                | ✅      | Sound-effects checkbox plus embedded countdown and successful mode-transition cues; implies `gui`  |
| `desktop-icons`        | ✅      | Hides icons when entering Gaming Mode and shows them when restoring Desktop Mode                    |
| `desktop-background`   | ✅      | Disables the wallpaper with a solid black background, then re-enables it on restore                 |
| `minimize-all-windows` | ✅      | Minimizes open windows when entering Gaming Mode only                                               |

Taskbar auto-hide is the core mode switch and is always compiled, even when every Cargo feature is disabled.

Some useful custom builds:

```powershell
# Silent GUI with taskbar handling and resolution profiles only
cargo build --release --package win11-borderless-gaming-desktop `
  --no-default-features --features gui

# GUI with sounds and selected desktop actions
cargo build --release --package win11-borderless-gaming-desktop `
  --no-default-features --features sound,desktop-icons,desktop-background
```

### Non-GUI compatibility mode

Build without `gui` to preserve the v1.1 one-shot behavior. Each execution:

1. reads the primary taskbar's current auto-hide state
2. switches to the opposite mode
3. applies every optional action compiled into that binary
4. exits immediately

There is no window, tray, countdown, saved GUI settings, resolution profile,
startup-at-login or transparency control, or interactive error report in this
mode.

One-shot build with every desktop action:

```powershell
cargo build --release --package win11-borderless-gaming-desktop `
  --no-default-features `
  --features desktop-icons,desktop-background,minimize-all-windows
```

One-shot build with selected actions:

```powershell
cargo build --release --package win11-borderless-gaming-desktop `
  --no-default-features `
  --features desktop-icons,desktop-background
```

Minimal taskbar-only toggle:

```powershell
cargo build --release --package win11-borderless-gaming-desktop `
  --no-default-features
```

### Developer power-ups

Regenerate the runtime media, build in release mode with every feature, and
launch the GUI:

```powershell
cargo xtask run
```

Regenerate the runtime media, build in release mode with every desktop
action, launch the one-shot build, and omit `gui`:

```powershell
cargo xtask run --no-gui
```

Regenerate the compact embedded artwork and procedural sound cues plus the
application and tray icons:

```powershell
cargo xtask assets
```

The pipeline downsizes mode wordmarks in linear light with premultiplied alpha,
and the GUI uses linear mipmaps so their edges stay smooth at display size.
The GUI embeds its compact media from `assets/runtime`; `app.ico` supplies the
executable icon. The `sound` feature embeds the three dependency-free cues,
which total about 84 KiB; builds without `sound` contain neither those cues nor
the Windows playback code. `gui.png` and `readme-logo-master.png` are
documentation-only and do not affect executable size.

Run the test suite and the repository checks before sending your build into ranked:

```powershell
cargo test --workspace --all-features
cargo xtask check all
```

`cargo xtask check all` covers formatting, strict Clippy, dependency auditing, and typo checks. The Rust tests are the separate first command.

### How the mode switch works

Windows taskbar auto-hide is the source of truth: disabled means Desktop Mode, enabled means Gaming Mode. A GUI build reflects external changes to that setting and applies selected actions from the control panel. A non-GUI build simply flips that state, applies its compiled actions, and exits.

Resolution modes are collected from the primary monitor at GUI startup, deduplicated, and sorted by height then width from largest to smallest. Modes taller than the monitor's native height are filtered out. Windows keeps or chooses compatible refresh-rate, color-depth, and orientation values when applying the selected width and height.

**Start at login** uses the current user's
`Software\Microsoft\Windows\CurrentVersion\Run` registry key. When **Start
minimized** is selected, the login command adds the app's internal `--minimized`
argument. Window transparency is applied with Windows layered-window alpha and
is capped at 80% to keep the control panel recoverable.

### License

Borderless Gaming Desktop is free software released under the [GNU General Public License v3.0 or later](LICENSE).

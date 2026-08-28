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
distraction-free **Gaming Mode**. The app records your Desktop resolution on its
first start; choose a narrower Gaming resolution and clear distractions in one
click. The portable app needs no installer or terminal, and it makes no surprise
desktop changes just because you launched it. It can also start with Windows,
keep its automatic login launch minimized, and tune whole-window transparency.

### Ready, player one?

1. [Download the latest portable executable from GitHub Releases](https://github.com/syl20bnr/win11-borderless-gaming-desktop/releases/latest).
2. Put the `.exe` wherever you like and double-click it. That is the whole installation.
3. Pick your Gaming resolution and actions, choose the application behavior, then hit the big purple button.

<p align="center">
  <img
    src="https://raw.githubusercontent.com/syl20bnr/win11-borderless-gaming-desktop/refs/heads/main/crates/win11-borderless-gaming-desktop/assets/gui.png"
    width="531"
    alt="Borderless Gaming Desktop in Gaming Mode with gaming options, startup, and transparency controls"
  >
</p>

### Build your Gaming Mode loadout

The portable release is ready to:

- optionally hide the Windows taskbar with auto-hide
- hide desktop icons
- swap the desktop wallpaper for solid black
- minimize open windows
- switch your primary Ultrawide monitor to your chosen Gaming resolution

Every optional desktop action has its own checkbox. **Auto-hide the taskbar** is
selected by default, while sound effects have a separate **Enable sounds**
checkbox and are also enabled by default. Selecting a Gaming resolution never
changes it immediately; it is applied only when you activate Gaming Mode. The
resolution captured on the app's first start is restored when you return to
Desktop Mode. Your monitor's preferred resolution is labeled `(native)`, and the
Gaming resolution combo remains sorted from largest to smallest by height and
then width. While Gaming Mode is active, its desktop-action checkboxes are locked
until you restore Desktop Mode; **Enable sounds** remains adjustable.

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
| **Taskbar**    | Original setting restored    | Auto-hide when selected       |
| **Desktop**    | Icons and wallpaper restored | Your selected cleanup actions |
| **Resolution** | Captured on first start       | Your selected profile         |
| **Status LED** | Cool gray                    | Bright green                  |

Press **Activate gaming mode** and the button finishes its click animation before
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
restores the selected desktop actions, the Desktop resolution, and, when
selected, the exact taskbar setting captured during activation. When sounds are
enabled, a compact power-down cue confirms the switch. Minimized windows stay
minimized; the app will not unexpectedly reopen them over your post-game screen.

### Your loadout remembers you

Action checkboxes, the sound-effects preference, the captured Desktop resolution,
your Gaming resolution, the current app-owned mode, the minimized-start
preference, and window transparency are saved automatically.
**Start at login** reflects the real Windows login entry instead of keeping a
separate stale flag. Close the app, reboot, come back tomorrow, your choices are
still equipped.

While Gaming Mode is active, the app also preserves the activation snapshot and,
when taskbar auto-hide is selected, its original setting, so relaunching does not
lose the information needed for an exact Desktop Mode restore. A failed
resolution target is preserved too, so **Retry resolution** remains available
after a relaunch.

Gaming Mode uses its own persistent state instead of inferring the mode from
Windows taskbar auto-hide. This keeps the mode reliable when the taskbar action
is turned off or when another program changes the taskbar setting. When taskbar
auto-hide is selected, activation records the current Windows setting before
enabling it, and Desktop Mode restores that exact original setting.

Only one app instance can run at a time, preventing two windows from competing
over the same mode snapshot. Launching the app again brings the existing window
forward and exits the duplicate process.

### Living in the system tray

- The custom close button and `Alt+F4` hide the app in the system tray without leaving it in the taskbar. The first close explains this once.
- Double-click the tray icon to activate Gaming Mode or restore Desktop Mode.
- Right-click it to **Open**, **Activate Gaming Mode / Restore Desktop Mode**, or **Quit**; the same menu includes a disabled summary of the configured Gaming options.
- The tray LED mirrors the app: gray for Desktop, blinking orange during the countdown and while either mode is being applied, and green for Gaming.
- A regular Windows minimize remains a regular taskbar minimize.

> [!TIP]
> **Quit** means quit, it does not silently change your mode. If you are still in Gaming Mode, restore Desktop Mode first when you want the everyday desktop back.

## Advanced Users and Developers

The project ships as one full desktop app with its control panel, system tray,
sound effects, resolution profiles, and Gaming Mode actions included.

### Build or install the app

Requirements: **Windows 11** and **Rust 1.94 or newer**.

Install the published crates.io app:

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

### Developer power-ups

Regenerate the runtime media, build in release mode, and launch the app:

```powershell
cargo xtask run
```

Regenerate the compact embedded artwork and procedural sound cues plus the
application and tray icons:

```powershell
cargo xtask assets
```

The pipeline downsizes mode wordmarks in linear light with premultiplied alpha,
and the app uses linear mipmaps so their edges stay smooth at display size. It
embeds its compact media from `assets/runtime`; `app.ico` supplies the executable
icon, and the three dependency-free sound cues total about 84 KiB. `gui.png` and
`readme-logo-master.png` are documentation-only and do not affect executable
size.

Run the test suite and the repository checks before sending your build into ranked:

```powershell
cargo test --workspace
cargo xtask check all
```

`cargo xtask check all` covers formatting, strict Clippy, dependency auditing, and typo checks. The Rust tests are the separate first command.

### How the mode switch works

The app owns the Gaming/Desktop mode as a persistent boolean; Windows taskbar
auto-hide no longer determines it. Activating Gaming Mode snapshots the selected
actions and the taskbar's current auto-hide setting. If **Auto-hide the taskbar**
is selected, the app enables it for Gaming Mode and restores the captured setting
when returning to Desktop Mode. The other selected desktop actions follow the
same activation and restoration flow, while minimize-windows runs only during
activation.

Resolution modes are collected from the primary monitor at startup, deduplicated,
and sorted by height then width from largest to smallest in the Gaming resolution
combo. Modes taller than the monitor's native height are filtered out. The
current resolution is captured as the hidden Desktop restore profile the first
time settings are initialized. Choosing a Gaming resolution does not apply it
immediately: activation applies it, and restoration returns to the captured
Desktop profile. Windows keeps or chooses compatible refresh-rate, color-depth,
and orientation values when applying the selected width and height. The tray icon
and its default double-click action follow the persistent mode, while its context
menu shows the currently configured Gaming options.

**Start at login** uses the current user's
`Software\Microsoft\Windows\CurrentVersion\Run` registry key. When **Start
minimized** is selected, the login command adds the app's internal `--minimized`
argument. Saved window transparency is restored automatically as the native
window becomes visible, uses Windows layered-window alpha, and is capped at 80%
to keep the control panel recoverable.

### License

Borderless Gaming Desktop is free software released under the [GNU General Public License v3.0 or later](LICENSE).

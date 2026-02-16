# Lumbus

Highlight the mouse pointer across **all** displays with a configurable circle. Shows a bold **L** (left click) or **R** (right click) to visualise clicks—great for **presentations**, **screen recordings**, and **remote support**. The overlay stays on top without stealing focus.

**Cross-platform:** macOS and Windows.

> This software is not affiliated with Apple or Microsoft. Use at your own risk.

---

## Features

| Feature | macOS | Windows |
|---------|-------|---------|
| Multi-display overlay | Yes | Yes |
| Smooth pointer tracking (~60 FPS) | Yes | Yes |
| Click indicators (L/R or I/D) | Yes | Yes |
| Configurable radius, border, color | Yes | Yes |
| Fill transparency | Yes | Yes |
| Global hotkeys | Yes | Yes |
| System tray / Status bar | Yes | Yes |
| Settings persistence | NSUserDefaults | JSON |
| Localisation (EN/ES) | Yes | Yes |

### Click Indicators
- **Default:** Circle at cursor
- **Left click:** **L** (English) / **I** (Spanish)
- **Right click:** **R** (English) / **D** (Spanish)

### Global Hotkeys

| Action | macOS | Windows |
|--------|-------|---------|
| Toggle overlay | `Ctrl+A` | `Ctrl+Shift+A` |
| Open Settings | `Ctrl+,` | `Ctrl+Shift+S` |
| Show Help | `Cmd+Shift+H` | `Ctrl+Shift+H` |
| Quit | `Ctrl+Shift+X` | `Ctrl+Shift+Q` |

---

## Installation

### macOS

#### Option 1: Download DMG (recommended)

Download the latest `Lumbus-x.x.x.dmg` from [Releases](https://github.com/restevean/lumbus/releases), open it, and drag `Lumbus.app` to your Applications folder.

> **First launch:** Since the app is not signed with an Apple Developer ID, macOS will show a warning. Right-click the app → Open → Open to bypass Gatekeeper.

#### Option 2: Build from source

```bash
git clone https://github.com/restevean/lumbus.git
cd lumbus
./scripts/build-app.sh
cp -R target/release/Lumbus.app /Applications/
```

**Requirements:**
- macOS 10.13+
- Rust stable (1.70+)
- Grant **Accessibility** and **Input Monitoring** permissions when prompted

### Windows

#### Option 1: Download executable (recommended)

Download the latest `lumbus.exe` from [Releases](https://github.com/restevean/lumbus/releases):
- `lumbus-x.x.x-windows-x64.exe` for Intel/AMD (64-bit)
- `lumbus-x.x.x-windows-arm64.exe` for ARM64

> **First launch:** Windows SmartScreen may block the unsigned executable. Click "More info" → "Run anyway", or right-click → Properties → Unblock.

#### Option 2: Build from source

```powershell
git clone https://github.com/restevean/lumbus.git
cd lumbus
cargo build --release
# Executable at target\release\lumbus.exe
```

**Requirements:**
- Windows 10/11
- Rust stable (1.70+)
- Visual Studio Build Tools (for MSVC linker)

---

## Usage

1. **Launch** the app. A circle appears following your cursor.
2. **Toggle** visibility with the hotkey (`Ctrl+A` on macOS, `Ctrl+Shift+A` on Windows).
3. **Configure** via Settings hotkey (`Ctrl+,`) or tray/status bar menu.
4. **Click** to see L/R indicators (or I/D in Spanish).

### Settings Panel

- **Language:** English / Español
- **Radius (px):** Slider (5-200, snaps to 5)
- **Border (px):** Slider (1-20, snaps to 1)
- **Color:** Color picker (macOS: Hex editable, Windows: system dialog)
- **Fill Transparency (%):** Slider (0-100, snaps to 5)
  - 0% = fully opaque fill
  - 100% = no fill (transparent)

---

## Visuals

- **Circle:** Configurable stroke color/width; fill uses same color with configurable alpha.
- **Letters (L/R or I/D):** Bold font, same stroke and fill as circle, centered on cursor, height ≈ 1.5× circle diameter.

---

## How It Works

### macOS
- One borderless, transparent `NSWindow` per screen, always-on-top.
- Pointer from `NSEvent.mouseLocation`, converted to view coordinates.
- Drawing: `NSBezierPath` for circle, `CTFontCreatePathForGlyph` for letters.
- Hotkeys: Carbon `RegisterEventHotKey` (no system beep).
- Persistence: `NSUserDefaults`.

### Windows
- One layered window (`WS_EX_LAYERED`) spanning all monitors.
- Pointer from `GetCursorPos`.
- Drawing: Direct2D with `UpdateLayeredWindow` for per-pixel alpha.
- Hotkeys: `RegisterHotKey`.
- Persistence: JSON in `%APPDATA%\Lumbus\config.json`.

---

## Code Structure

```
src/
├── main.rs              # Entry point (platform dispatch)
├── macos_main.rs        # macOS orchestrator (~185 lines)
├── windows_main.rs      # Windows orchestrator (~285 lines)
├── lib.rs               # Shared helpers
├── events/              # Cross-platform event bus
├── model/               # Cross-platform state & constants
└── platform/
    ├── macos/           # macOS-specific code
    │   ├── ffi/         # Carbon, CoreText, Cocoa bindings
    │   ├── ui/          # Overlay (view.rs), settings, dialogs, status bar
    │   ├── input/       # Hotkeys, mouse monitors
    │   └── storage/     # NSUserDefaults
    └── windows/         # Windows-specific code
        ├── app/         # State management
        ├── ffi/         # Win32 bindings
        ├── ui/          # Overlay (renderer.rs), settings, dialogs, tray
        ├── input/       # Hotkeys, mouse hooks
        └── storage/     # JSON config
```

**Tests:** 65 unit tests (`cargo test`)

---

## Troubleshooting

### macOS
- **Shortcuts don't work:** Grant Accessibility and Input Monitoring permissions in System Preferences → Security & Privacy.
- **Overlay not visible:** Toggle with `Ctrl+A`.

### Windows
- **SmartScreen blocks exe:** Click "More info" → "Run anyway".
- **No overlay visible:** Check system tray icon, toggle with `Ctrl+Shift+A`.
- **Settings not saving:** Ensure write access to `%APPDATA%\Lumbus\`.

---

## Roadmap

- [x] macOS support (production-ready)
- [x] Windows support (production-ready)

---

## License

Apache License 2.0. See [LICENSE](LICENSE) file.

---

## Acknowledgments

- **macOS:** Built with `objc2`, `objc2-foundation`, `block2`, and Core* frameworks.
- **Windows:** Built with `windows-rs` (official Microsoft Rust bindings), Direct2D, DirectWrite.

Tested on macOS 14+ and Windows 11. Works with ANSI and ISO keyboards.

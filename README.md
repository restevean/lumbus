# Lumbus (macOS)

Highlight the mouse pointer across **all** macOS displays with a configurable circle. Optionally show a bold **L** (left click) or **R** (right click) to visualise clicks—great for **presentations**, **screen recordings**, and **remote support**. The overlay stays on top without stealing focus or causing the system beep. 🔨🤖🔧

> ⚠️ This software was created via vibe coding and is not affiliated with Apple or any other company. Use at your own risk.

---

## ✨ Features

- **Multi-display overlay**
    - One transparent, borderless window **per display**, above menus and Dock.
    - Doesn’t take focus or interfere with the active app.
- **Smooth pointer tracking** (~60 FPS using Cocoa screen coordinates).
- **Click indicators**
    - Default: **circle** at the cursor.
    - **Left mouse down** → bold **L**.
    - **Right mouse down** → bold **R**.
    - Letters use the **same border width** and **same fill transparency** as the circle.
- **Live configuration**
    - **Circle radius (px)** — via slider (snap in steps of **5**)
    - **Border thickness (px)** — via slider (snap in steps of **1**)
    - **Colour** via system colour picker (**NSColorWell**) and editable **Hex** (`#RRGGBB` or `#RRGGBBAA`)
    - **Fill transparency (%)** — via slider (snap in steps of **5**), `0` (opaque) → `100` (fully transparent)
    - Numeric fields for radius/border/transparency are **display-only** (read-only). Adjust via sliders.
    - Sliders have **no tick marks**, but values still **snap** to the increments above.
    - Changes apply **instantly** and are **persisted**.
- **Global hotkeys (no beep)**
    - `Ctrl` + `A` → **Toggle overlay visibility**
    - `⌘` + `,` → **Open Settings**
    - `⌘` + `;` → **Open Settings** (alternate)
    - `⌘` + `Shift` + `H` → **Show Help** (keyboard shortcuts)
    - `Ctrl` + `Shift` + `X` → **Quit** (with confirmation)
- **Help overlay** — Press `⌘+Shift+H` to show all keyboard shortcuts in a centered overlay
- **Persistence** via `NSUserDefaults` (restored on launch)

---

## 🖼️ Visuals

- **Circle:** configurable stroke colour/width; fill uses the same colour with configurable alpha.
- **L/R letters:** CoreText (glyph → CGPath → NSBezierPath), same stroke and fill alpha as the circle, centred on the cursor, height ≈ `1.5 × circle diameter`.

---

## 📦 Requirements

- macOS **10.13+** (uses `NSBezierPath` + CGPath bridging and Accessibility prompt API)
- Rust stable (1.70+ recommended)
- `Cargo.toml`:

```toml
[dependencies]
cocoa = "0.25"
objc = "0.2"
block = "0.1"
# core-graphics = "0.24"   # Optional; not strictly required by current code
```

> 🔐 Hotkeys use Carbon and generally **don't require** Accessibility.
>  🖱️ Mouse monitors use `NSEvent::addGlobalMonitorForEventsMatchingMask`. If prompted, allow **Input Monitoring** and/or **Accessibility** for the app (or for your IDE if you run from there).

------

## 📥 Installation

### Option 1: Download DMG installer (recommended)

Download the latest `Lumbus-x.x.x.dmg` from [Releases](https://github.com/restevean/lumbus/releases), open it, and drag `Lumbus.app` to your Applications folder.

> ⚠️ **First launch**: Since the app is not signed with an Apple Developer ID, macOS will show a warning. Right-click the app → Open → Open to bypass Gatekeeper.

### Option 2: Build from source

```bash
# Clone the repository
git clone https://github.com/restevean/lumbus.git
cd lumbus

# Build the .app bundle
make bundle

# Install to ~/Applications
make install-user

# Or install to /Applications (may require sudo)
make install
```

### Available make targets

| Command | Description |
|---------|-------------|
| `make bundle` | Build release `.app` bundle |
| `make install-user` | Install to `~/Applications` |
| `make install` | Install to `/Applications` |
| `make icons` | Regenerate app icon from `resources/icons/source.png` |
| `make open` | Build and launch the app |
| `make clean` | Clean build artifacts |

------

## ▶️ Usage

1. Build & run:

   ```bash
   cargo run --profile dev
   ```

2. Toggle overlay with **Ctrl + A** (works while the app is running; it doesn’t need to be frontmost).

3. Open **Settings** with **⌘ + ,** or **⌘ + ;**.

4. Adjust **radius**, **border**, **colour** (picker or Hex), and **fill transparency**.

    - Numeric boxes show the value but are **not editable**.
    - Use sliders (snap to valid steps).
    - **Hex** remains editable.

5. Click behaviour:

    - **Left down** → shows **L**
    - **Right down** → shows **R**
    - On release → reverts to **circle**

6. Quit with **Ctrl + Shift + X**.
   A confirmation dialog appears with **Cancel** (default) and **Quit**.

    - **Enter/Return** activates the highlighted default button.
    - **Esc** cancels and closes the dialog.
    - **Tab** cycles focus between buttons.

------

## ⚙️ Settings Panel

- **Language**: English / Español
- **Radius (px)** — **read-only** value + slider (`5..200`, snap `5`)
- **Border (px)** — **read-only** value + slider (`1..20`, snap `1`)
- **Colour** — colour well + **Hex** (`#RRGGBB` or `#RRGGBBAA`, **Hex is editable**)
- **Fill Transparency (%)** — **read-only** value + slider (`0..100`, snap `5`)
  `100` = no fill (fully transparent), `0` = fully opaque

**Shortcuts in Settings:**

- **Enter/Return** → activates **Close**
- **Esc** → closes Settings
- Initial focus is on the **Close** button.

------

## ⌨️ Global Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl` + `A` | Toggle overlay |
| `⌘` + `,` | Open Settings |
| `⌘` + `;` | Open Settings (alternate) |
| `⌘` + `Shift` + `H` | Show Help |
| `Ctrl` + `Shift` + `X` | Quit (with confirmation) |

Implemented with **Carbon HotKeys** (no beep) and a local key monitor for extra reliability while windows are key.

------

## 🧠 How It Works

- One borderless, transparent `NSWindow` **per screen** (`NSScreen.screens`), always-on-top level.
- Pointer from **Cocoa** (`NSEvent.mouseLocation`), converted **screen → window → view**.
- Drawing:
    - **Circle:** `NSBezierPath::bezierPathWithOvalInRect` → `fill` (alpha from transparency) + `stroke`
    - **L/R:** `CTFontCreatePathForGlyph` → `CGPath` → `NSBezierPath` → `fill` + `stroke`
- **Hotkeys:** `RegisterEventHotKey` + `InstallEventHandler` (Carbon)
- **Mouse:** `NSEvent::addGlobalMonitorForEventsMatchingMask` (left/right down/up, move)
- **Persistence:** `NSUserDefaults`
- **Permissions:** On first run we invoke `AXIsProcessTrustedWithOptions` to prompt Accessibility if needed.

------

## 🧪 Troubleshooting

- **Shortcuts beep or don’t trigger:** Use **Ctrl+A**, **⌘+,**, **⌘+;**, or **Ctrl+Shift+X**. Other apps may intercept overlapping shortcuts—adjust their settings if necessary.
- **Overlay not following the cursor:** Ensure **Input Monitoring** and/or **Accessibility** are enabled for your binary (or for your IDE if launching from it).
- **Hex field layout:** It sits to the right of the “Hex” label; tweak the constants if you change window width.

------

## 🗂️ Code Structure

```
src/
├── main.rs              # Entry point + overlay view registration
├── lib.rs               # Pure helpers (clamp, color_to_hex, parse_hex_color, tr_key)
│
├── events/              # Event bus system
│   ├── bus.rs           # EventBus implementation
│   ├── global.rs        # Global publish/subscribe functions
│   └── types.rs         # AppEvent enum definitions
│
├── handlers/            # Event handlers
│   └── dispatcher.rs    # Main event dispatcher
│
├── ffi/                 # FFI bindings encapsulated
│   ├── carbon.rs        # Carbon Event Manager (hotkeys)
│   ├── coretext.rs      # CoreText (glyph rendering)
│   ├── coregraphics.rs  # CoreGraphics/CoreFoundation
│   ├── accessibility.rs # Accessibility permissions
│   ├── cocoa_utils.rs   # NSString, display_id, mouse position helpers
│   └── types.rs         # Type aliases (ViewId, WindowId, etc.)
│
├── model/               # Pure domain logic (testable, no FFI)
│   ├── constants.rs     # Default values, pref keys, limits
│   ├── app_state.rs     # OverlayState struct with validation
│   └── preferences.rs   # NSUserDefaults load/save
│
├── input/               # Input handling
│   ├── hotkeys.rs       # Carbon hotkey install/uninstall
│   ├── observers.rs     # Wake/space/termination observers
│   ├── mouse_monitors.rs    # Global mouse event monitors
│   └── keyboard_monitors.rs # Local Ctrl+A monitor
│
├── ui/
│   ├── overlay/         # Overlay drawing
│   │   └── drawing.rs   # draw_circle, draw_letter, DrawParams
│   ├── settings/        # Settings window
│   │   └── window.rs    # open/close settings, controls
│   ├── dialogs/         # Dialog windows
│   │   ├── quit_dialog.rs   # Quit confirmation
│   │   └── help_overlay.rs  # Help overlay with keyboard shortcuts
│   └── status_bar.rs    # Menu bar icon and dropdown menu
│
└── app/                 # Shared application helpers
    └── helpers.rs       # apply_to_all_views, sync_visual_prefs
```

**Tests:** 64 unit tests across `tests/` and inline modules

------

## 🛣️ Roadmap

No planned features at this time. Feel free to open an issue with suggestions!

------

## 📄 Licence

Apache License 2.0. See [LICENSE](LICENSE) file.

------

## 🙌 Acknowledgments

Built with `cocoa`, `objc`, `block`, and a sprinkle of Core* frameworks via FFI.
Tested on macOS with **ANSI** and **ISO** keyboards (`⌘+,` and `⌘+;` cover both). ✔️
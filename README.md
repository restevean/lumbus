# Mouse Highlighter (macOS)

Highlight the mouse pointer across **all** macOS displays with a configurable circle. Optionally show a bold **L** (left click) or **R** (right click) to visualize clicks—great for **presentations**, **screen recordings**, and **remote support**. The overlay stays on top without stealing focus or causing the system beep. 🔨🤖🔧

------

## ✨ Features

- **Full-screen overlay (all displays)**
  - Borderless, transparent window above menus and Dock.
  - Doesn’t take focus or interfere with the active app.
- **Smooth pointer tracking** (~60 FPS using Cocoa screen coordinates).
- **Click indicators**
  - Default: **circle** at the cursor.
  - **Left mouse down** → bold **L** centered.
  - **Right mouse down** → bold **R** centered.
  - Letters use the **same border width** and **same fill transparency** as the circle.
- **Live configuration**
  - **Circle radius (px)**
  - **Border thickness (px)**
  - **Color** via system color picker (**NSColorWell**) and editable **Hex** (`#RRGGBB` or `#RRGGBBAA`)
  - **Fill transparency (%)**: `0` (opaque) → `100` (fully transparent)
  - Changes apply **instantly**.
- **Global hotkeys (no beep)**
  - `Ctrl` + `A` → **Toggle overlay visibility**
  - `⌘` + `,` → **Open Settings**
  - `⌘` + `;` → **Open Settings** (alternate)
- **Persistence** via `NSUserDefaults` (restored on launch)

------

## 🖼️ Visuals

- **Circle:** configurable stroke color/width; fill uses same color with configurable alpha.
- **L/R letters:** CoreText (glyph → CGPath → NSBezierPath), same stroke and fill alpha as the circle, centered on the cursor, height ≈ `1.5 × circle diameter`.

------

## 📦 Requirements

- macOS **10.13+** (uses `NSBezierPath` with `CGPath`)
- Rust stable (1.70+ recommended)
- `Cargo.toml`:

```
toml


Copiar
[dependencies]
cocoa = "0.25"
objc = "0.2"
block = "0.1"
core-graphics = "0.24"
```

> ⚠️ Hotkeys use Carbon and typically **don’t require** Accessibility.
>  Mouse monitors use `NSEvent::addGlobalMonitorForEventsMatchingMask`. If prompted, allow **Input Monitoring** / **Accessibility** for the app.

------

## ▶️ Usage

1. Build & run:

   ```
   bash
   
   
   Copiar
   cargo run --profile dev
   ```

2. Toggle overlay with **Ctrl + A**.

3. Open **Settings** with **⌘ + ,** or **⌘ + ;**.

4. Adjust radius, border, color (picker or Hex), and fill transparency. Changes are **live** and **saved** automatically.

5. Click behavior:

   - **Left down** → shows **L**
   - **Right down** → shows **R**
   - On release → reverts to **circle**

------

## ⚙️ Settings Panel

- **Radius (px)** — numeric field + slider (`5..200`)
- **Border (px)** — numeric field + slider (`1..20`)
- **Color** — color well + **Hex** (`#RRGGBB` or `#RRGGBBAA`)
- **Fill Transparency (%)** — numeric field + slider (`0..100`)
   `100` = no fill (fully transparent), `0` = fully opaque

All controls are editable and synchronized.

------

## ⌨️ Global Shortcuts

- `Ctrl` + `A` → Toggle overlay
- `⌘` + `,` → Open Settings
- `⌘` + `;` → Open Settings (alternate)

Implemented with **Carbon HotKeys** (no beep, no focus needed).

------

## 🧠 How It Works

- Borderless, transparent `NSWindow` spans the **union of all screens** (`NSScreen.screens`).
- Pointer from **Cocoa** (`NSEvent.mouseLocation`), converted **screen → window → view**.
- Drawing:
  - **Circle:** `NSBezierPath::bezierPathWithOvalInRect` → `fill` (alpha from transparency) + `stroke`
  - **L/R:** `CTFontCreatePathForGlyph` → `CGPath` → `NSBezierPath` → `fill` + `stroke`
- **Hotkeys:** `RegisterEventHotKey` + `InstallEventHandler` (Carbon)
- **Mouse:** `NSEvent::addGlobalMonitorForEventsMatchingMask` (left/right down/up)
- **Persistence:** `NSUserDefaults`

------

## 🧪 Troubleshooting

- **System beep on shortcut:** Use **Ctrl+A** / **⌘+,** / **⌘+;**. Other apps might intercept; adjust their shortcuts.
- **L/R not showing:** Ensure overlay **visible** (Ctrl+A). Global mouse-capture tools may block monitors.
- **Hex field layout:** Field appears right after “Hex” with a right margin; adjust constants if you change window width.

------

## 🗂️ Code Structure (high level)

- **`main.rs`**
  - FFI: Carbon, CoreText, CoreGraphics, CoreFoundation
  - Helpers: `clamp`, `color_to_hex`, `parse_hex_color`, NSUserDefaults
  - Overlay `NSWindow` + `CustomView` (state, drawing, settings actions)
  - Hotkey registration/unregistration
  - Global mouse monitors
  - Settings window (live-synced controls)

------

## 🛣️ Roadmap

- Option to show **only** the circle (no letters)
- Short “flash” on click instead of letters
- Presets for color/size
- Entry/exit animations

------

## 📄 License

MIT (or your preferred license). Add a `LICENSE` file.

------

## 🙌 Acknowledgments

Built with `cocoa`, `objc`, `block`, and `core-graphics` crates.
 Tested on macOS with **ANSI** and **ISO** keyboards (`⌘+,` and `⌘+;` cover both). ✔️

------

### Optional: create the file and a ZIP locally (safe fences)

**macOS/Linux (bash, one command):**

```
bash


Copiar
python3 - <<'PY'
import zipfile, pathlib
readme = pathlib.Path("README.md").read_text(encoding="utf-8") if pathlib.Path("README.md").exists() else """PASTE THIS WHOLE README CONTENT HERE"""
pathlib.Path("README.md").write_text(readme, encoding="utf-8")
with zipfile.ZipFile("mouse_highlighter_readme.zip","w",zipfile.ZIP_DEFLATED) as zf:
    zf.writestr("README.md", readme)
print("✅ Created README.md and mouse_highlighter_readme.zip")
PY
```

**Windows (PowerShell, one command):**

```
powershell


Copiar
python - << 'PY'
import zipfile, pathlib
readme_path = pathlib.Path('README.md')
if readme_path.exists():
    readme = readme_path.read_text(encoding='utf-8')
else:
    readme = """PASTE THIS WHOLE README CONTENT HERE"""
readme_path.write_text(readme, encoding='utf-8')
with zipfile.ZipFile('mouse_highlighter_readme.zip','w',zipfile.ZIP_DEFLATED) as zf:
    zf.writestr('README.md', readme)
print("✅ Created README.md and mouse_highlighter_readme.zip")
PY
```


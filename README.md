# Mouse Highlighter (macOS)

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
    - `Ctrl` + `Shift` + `Q` → **Quit** (with confirmation)
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

> 🔐 Hotkeys use Carbon and generally **don’t require** Accessibility.
>  🖱️ Mouse monitors use `NSEvent::addGlobalMonitorForEventsMatchingMask`. If prompted, allow **Input Monitoring** and/or **Accessibility** for the app (or for your IDE if you run from there).

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

6. Quit with **Ctrl + Shift + Q**.
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

- `Ctrl` + `A` → Toggle overlay
- `⌘` + `,` → Open Settings
- `⌘` + `;` → Open Settings (alternate)
- `Ctrl` + `Shift` + `Q` → **Quit** (with confirmation; **Esc** cancels)

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

- **Shortcuts beep or don’t trigger:** Use **Ctrl+A**, **⌘+,**, **⌘+;**, or **Ctrl+Shift+Q**. Other apps may intercept overlapping shortcuts—adjust their settings if necessary.
- **Overlay not following the cursor:** Ensure **Input Monitoring** and/or **Accessibility** are enabled for your binary (or for your IDE if launching from it).
- **Hex field layout:** It sits to the right of the “Hex” label; tweak the constants if you change window width.

------

## 🗂️ Code Structure (high level)

- **`main.rs`**
    - FFI: Carbon, CoreText, CoreGraphics, CoreFoundation, ApplicationServices (Accessibility prompt)
    - Helpers: `clamp`, `color_to_hex`, `parse_hex_color`, NSUserDefaults
    - Overlay `NSWindow` + `CustomView` (state, drawing, settings actions)
    - Hotkey registration/unregistration (+ keep-alive + wake/space observers)
    - Global mouse monitors and local key monitors
    - Settings window (live-synced sliders, read-only numeric labels, editable Hex)

------

## 🛣️ Roadmap

- Option to show **only** the circle (no letters)
- Short “flash” on click instead of letters
- Presets for colour/size
- Entry/exit animations

------

## 📄 Licence

MIT (or your preferred licence). Add a `LICENSE` file.

------

## 🙌 Acknowledgments

Built with `cocoa`, `objc`, `block`, and a sprinkle of Core* frameworks via FFI.
Tested on macOS with **ANSI** and **ISO** keyboards (`⌘+,` and `⌘+;` cover both). ✔️
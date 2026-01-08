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
    - Numeric fields for radius/bTested on macOS with **ANSI** and **ISO** keyboards (`⌘+,` and `⌘+;` cover both). ✔️
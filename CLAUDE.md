# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Communication Preferences

- **Use technical jargon** (milestone, sprint, refactor, etc.) - it helps the user learn industry terminology
- Explain new terms when they appear for the first time

## Project Overview

**Lumbus** is a macOS-native application written in Rust that highlights the mouse pointer across all displays with a configurable circle and click indicators (L/R). Built using Cocoa FFI bindings (`cocoa`, `objc`, `block` crates) and low-level Carbon/CoreText/CoreGraphics APIs.

This is a **presentation/screen recording tool**, not a system utility—it creates transparent overlay windows that track the cursor without stealing focus.

## Build and Run Commands

```bash
# Development build and run
cargo run --profile dev

# Release build
cargo build --release

# Run tests
cargo test

# Run specific test
cargo test test_name

# Run with verbose output
cargo test -- --nocapture
```

## Architecture Overview

### Core Components

**`src/main.rs`**
- Application entry point and `CustomView` NSView subclass registration
- Hotkey event handler dispatch
- Main run loop with event bus polling

**`src/lib.rs`** (platform-agnostic)
- Pure helper functions: `clamp`, `color_to_hex`, `parse_hex_color`, `tr_key`
- Event bus system (`events/`) for decoupled communication
- Model layer (`model/`) with app state and preferences

**`src/platform/macos/`** (macOS-specific)
- `ffi/`: FFI bindings for Carbon, CoreText, CoreGraphics, Cocoa
- `input/`: Hotkey registration, mouse monitors, system observers
- `ui/`: Overlay drawing, settings window, dialogs, status bar
- `app/`: Helper functions for view management

**`src/handlers/`** (platform-agnostic logic)
- Event dispatcher that routes `AppEvent`s to appropriate handlers

### Architecture Patterns

1. **Modular platform separation**: Platform-specific code in `platform/macos/`, shared logic in `lib.rs` and `handlers/`.

2. **State management**: All application state lives in `CustomView` instance variables via Rust static `Box::into_raw` pattern. Accessed through `msg_send!` to Objective-C runtime.

3. **Drawing strategy**:
   - Circle: `NSBezierPath::bezierPathWithOvalInRect`
   - Letters (L/R): `CTFontCreatePathForGlyph` → CGPath → NSBezierPath conversion
   - Both use same stroke colour/width and fill alpha

4. **Persistence**: `NSUserDefaults` for all settings (radius, border, colour, transparency, language, visibility state)

5. **Hotkey mechanism**: Carbon `RegisterEventHotKey` + `InstallEventHandler` to avoid system beep. Keep-alive mechanism using workspace wake/screensaver observers.

6. **Multi-display support**: Enumerate `NSScreen.screens` and create one overlay window per screen, each with independent coordinate conversion.

## Specific Implementation Details

### Hotkeys (Carbon Event Manager)
- **Ctrl+A**: Toggle overlay visibility
- **⌘+,** / **⌘+;**: Open Settings (ANSI/ISO keyboard support)
- **Ctrl+Shift+X**: Quit with confirmation

Hotkeys use Carbon API (not NSEvent global monitors) to avoid triggering system beep. Handler installed on `GetApplicationEventTarget()`.

### Click Indicators
- Left mouse down → bold **L**
- Right mouse down → bold **R**
- Mouse up → revert to circle
- Letters rendered using CoreText glyphs, centred on cursor, height ≈ 1.5× circle diameter

### Settings Window
- All numeric fields (radius, border, transparency) are **read-only** labels
- Sliders snap to specific increments (radius: 5px, border: 1px, transparency: 5%)
- Hex colour field is **editable** and bidirectionally synced with NSColorWell
- Changes apply instantly via `msg_send!` to update overlay state and persist to NSUserDefaults

### Coordinate Conversion
Pointer obtained from `NSEvent.mouseLocation` (screen coordinates) → converted to each window's coordinate system → converted to view coordinates for drawing.

## Testing Notes

- All pure functions in `lib.rs` have corresponding tests in `tests/helpers.rs`
- No integration/UI tests (Cocoa UI testing is non-trivial)
- Test coverage: 100% for pure helpers, 0% for FFI/UI code in `main.rs`

## macOS Permissions

First run prompts for:
- **Accessibility** (via `AXIsProcessTrustedWithOptions`)
- **Input Monitoring** (for global mouse/keyboard monitors)

Grant these to the app (or to RustRover/IDE if running from development environment).

## Known Constraints

- macOS 10.13+ required (uses modern Cocoa/CoreGraphics APIs)
- ANSI/ISO keyboard support via dual hotkeys (⌘+, and ⌘+;)
- No CLI arguments; all configuration via GUI
- No external config files; all state in NSUserDefaults
- No support for Windows/Linux (pure macOS Cocoa)

## Development Context

This project was created via "vibe coding" (AI-assisted rapid development). Code structure reflects single-iteration development:
- Minimal refactoring/modularisation (single large `main.rs`)
- FFI-heavy with extensive unsafe blocks
- Focus on functionality over architectural elegance

When modifying: preserve the FFI patterns and state management approach. Test pure helpers thoroughly; UI/FFI changes require manual testing on macOS.

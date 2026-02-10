# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Communication Preferences

- **Use technical jargon** (milestone, sprint, refactor, etc.) - it helps the user learn industry terminology
- Explain new terms when they appear for the first time

## Project Overview

**Lumbus** is a cross-platform application written in Rust that highlights the mouse pointer across all displays with a configurable circle and click indicators (L/R). Currently supports **macOS** (production-ready) with **Windows support in development**.

The macOS version uses the `objc2` ecosystem (`objc2`, `objc2-foundation`, `block2` crates) and low-level Carbon/CoreText/CoreGraphics APIs.

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

The codebase follows a **platform-abstraction pattern** with conditional compilation (`#[cfg(target_os = "...")]`).

### Project Structure

```
src/
├── main.rs                     # Entry point with cfg gates (~28 lines)
├── macos_main.rs               # macOS application logic (~950 lines)
├── windows_main.rs             # Windows stub (TODO)
├── lib.rs                      # Shared helpers + re-exports
├── events/                     # Cross-platform event bus
│   ├── bus.rs                  # EventBus with publish/subscribe
│   ├── global.rs               # Global publish(), drain_events()
│   └── types.rs                # AppEvent enum definitions
├── model/                      # Cross-platform state
│   ├── app_state.rs            # OverlayState struct
│   └── constants.rs            # Config defaults, pref keys, limits
└── platform/
    ├── macos/                  # macOS implementation
    │   ├── app/                # Shared app helpers (apply_to_all_views)
    │   ├── ffi/                # FFI bindings (Carbon, CoreText, Cocoa)
    │   ├── handlers/           # Event dispatcher
    │   ├── input/              # Hotkeys, mouse monitors, observers
    │   ├── storage/            # NSUserDefaults persistence
    │   └── ui/                 # Overlay drawing, settings, dialogs, status bar
    └── windows/                # Windows scaffolding (TODO)
        ├── app/
        ├── ffi/
        ├── handlers/
        ├── input/
        ├── storage/
        └── ui/

tests/
├── helpers.rs                  # Tests for pure helpers from lib.rs
└── model_tests.rs              # Tests for OverlayState validation
```

### Core Components

**`src/main.rs`** (~28 lines)
- Minimal entry point with platform cfg gates
- Initialises event bus, then calls `macos_main::run()` or `windows_main::run()`

**`src/macos_main.rs`** (~950 lines)
- CustomView class registration (NSView subclass)
- Overlay window creation (one per display)
- Hotkey handler callback
- All macOS-specific application logic

**`src/lib.rs`**
- Pure helper functions: `clamp`, `color_to_hex`, `parse_hex_color`, `tr_key`
- Re-exports `events`, `model`, `platform` modules
- Backward-compatible `ffi` re-export for macOS

**`src/platform/macos/`** - macOS implementation
- `ffi/`: Carbon, CoreText, CoreGraphics, Cocoa bindings
- `ui/`: Overlay drawing, settings window, dialogs, status bar
- `input/`: Hotkeys, mouse monitors, system observers
- `storage/`: NSUserDefaults persistence
- `handlers/`: Event dispatcher

**`src/events/`** - Cross-platform event bus
- Decoupled publish/subscribe for hotkey events
- Thread-safe global access via `publish()`, `drain_events()`

**`src/model/`** - Cross-platform state
- `OverlayState`: Visual parameters (radius, color, transparency)
- `constants`: Default values, preference keys, validation limits

### Architecture Patterns

1. **Platform abstraction**: Entry point dispatches to platform-specific modules via cfg gates. Shared code (events, model) is platform-agnostic.

2. **FFI encapsulation**: All Cocoa/Carbon/CoreText FFI is in `platform/macos/ffi/`. No Objective-C `.m` files.

3. **State management**: Application state lives in `CustomView` instance variables (Objective-C runtime). Accessed through `msg_send!` macros.

4. **Event-driven architecture**: Hotkeys publish events to a global bus; timer-based dispatcher processes them on main thread.

5. **Drawing strategy**:
   - Circle: `NSBezierPath::bezierPathWithOvalInRect`
   - Letters (L/R): `CTFontCreatePathForGlyph` -> CGPath -> NSBezierPath
   - Both use same stroke colour/width and fill alpha

6. **Persistence**: `NSUserDefaults` for all settings (radius, border, colour, transparency, language)

7. **Multi-display support**: Enumerate `NSScreen.screens` and create one overlay window per screen.

## Specific Implementation Details

### Hotkeys (Carbon Event Manager)
- **Ctrl+A**: Toggle overlay visibility
- **Ctrl+,**: Open Settings (Ctrl instead of Cmd to avoid macOS system shortcut conflict)
- **Cmd+Shift+H**: Show Help
- **Ctrl+Shift+X**: Quit with confirmation

Hotkeys use Carbon API (not NSEvent global monitors) to avoid triggering system beep. Handler installed on `GetApplicationEventTarget()`.

### Click Indicators
- Left mouse down -> bold **L**
- Right mouse down -> bold **R**
- Mouse up -> revert to circle
- Letters rendered using CoreText glyphs, centred on cursor, height = 1.5x circle diameter

### Settings Window
- All numeric fields (radius, border, transparency) are **read-only** labels
- Sliders snap to specific increments (radius: 5px, border: 1px, transparency: 5%)
- Hex colour field is **editable** and bidirectionally synced with NSColorWell
- Changes apply instantly and persist to NSUserDefaults

### Coordinate Conversion
Pointer obtained from `NSEvent.mouseLocation` (screen coordinates) -> converted to each window's coordinate system -> converted to view coordinates for drawing.

## Testing Notes

- All pure functions in `lib.rs` have corresponding tests in `tests/helpers.rs`
- Model validation tested in `tests/model_tests.rs`
- Event bus tested in `src/events/bus.rs` (unit tests)
- No integration/UI tests (Cocoa UI testing is non-trivial)
- Total: 65 tests

## macOS Permissions

First run prompts for:
- **Accessibility** (via `AXIsProcessTrustedWithOptions`)
- **Input Monitoring** (for global mouse/keyboard monitors)

Grant these to the app (or to RustRover/IDE if running from development environment).

## Known Constraints

- macOS 10.13+ required (uses modern Cocoa/CoreGraphics APIs)
- Windows support in development (currently shows stub message)
- No CLI arguments; all configuration via GUI
- No external config files; all state in NSUserDefaults (macOS)

## Development Context

This project uses a modular, platform-abstracted architecture:
- Cross-platform: `events/`, `model/`, `lib.rs`
- Platform-specific: `platform/macos/`, `platform/windows/`
- Entry points: `macos_main.rs`, `windows_main.rs`

When modifying:
- **Cross-platform code**: Keep it free of FFI/platform dependencies
- **macOS code**: Preserve FFI patterns and state management approach
- **Testing**: Test pure helpers thoroughly; UI/FFI changes require manual testing

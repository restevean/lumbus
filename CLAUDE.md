# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Communication Preferences

- **Use technical jargon** (milestone, sprint, refactor, etc.) - it helps the user learn industry terminology
- Explain new terms when they appear for the first time

## Project Overview

**Lumbus** is a cross-platform application written in Rust that highlights the mouse pointer across all displays with a configurable circle and click indicators (L/R). Supports **macOS** and **Windows** (both production-ready).

- **macOS:** Uses the `objc2` ecosystem (`objc2`, `objc2-foundation`, `block2` crates) and Carbon/CoreText/CoreGraphics APIs.
- **Windows:** Uses `windows-rs` (official Microsoft crate) with Direct2D/DirectWrite for rendering.

This is a **presentation/screen recording tool**, not a system utility—it creates transparent overlay windows that track the cursor without stealing focus.

## Build and Run Commands

```bash
# Development build and run (current platform)
cargo run --profile dev

# Release build (current platform)
cargo build --release

# Cross-compile for Windows from macOS (requires target)
cargo build --release --target x86_64-pc-windows-msvc

# Run tests
cargo test

# Run specific test
cargo test test_name

# Build macOS .app bundle
./scripts/build-app.sh

# Build macOS DMG installer
./scripts/build-dmg.sh
```

## Architecture Overview

The codebase follows a **platform-abstraction pattern** with conditional compilation (`#[cfg(target_os = "...")]`).

### Project Structure

```
src/
├── main.rs                     # Entry point with cfg gates (~30 lines)
├── macos_main.rs               # macOS application logic (~950 lines)
├── windows_main.rs             # Windows application logic (~770 lines)
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
    └── windows/                # Windows implementation
        ├── app/                # App helpers
        ├── ffi/                # Win32 type definitions
        ├── handlers/           # (minimal, logic in windows_main.rs)
        ├── input/              # (minimal, hotkeys in windows_main.rs)
        ├── storage/            # JSON config persistence
        └── ui/                 # Settings window, dialogs, tray icon

tests/
├── helpers.rs                  # Tests for pure helpers from lib.rs
└── model_tests.rs              # Tests for OverlayState validation
```

### Core Components

**`src/main.rs`** (~30 lines)
- Minimal entry point with platform cfg gates
- Initialises event bus, then calls `macos_main::run()` or `windows_main::run()`

**`src/macos_main.rs`** (~950 lines)
- CustomView class registration (NSView subclass)
- Overlay window creation (one per display)
- Hotkey handler callback
- All macOS-specific application logic

**`src/windows_main.rs`** (~770 lines)
- Direct2D/DirectWrite initialization
- Layered window for transparent overlay
- Global hotkeys and mouse hooks
- Cursor tracking timer

**`src/lib.rs`**
- Pure helper functions: `clamp`, `color_to_hex`, `parse_hex_color`, `tr_key`
- Re-exports `events`, `model`, `platform` modules

### Platform-Specific Details

#### macOS
- `ffi/`: Carbon, CoreText, CoreGraphics, Cocoa bindings
- `ui/`: Overlay drawing, settings window, dialogs, status bar
- `input/`: Hotkeys (Carbon), mouse monitors (NSEvent), observers
- `storage/`: NSUserDefaults persistence
- `handlers/`: Event dispatcher

#### Windows
- `ui/settings/`: Win32 settings dialog with sliders and combobox
- `ui/dialogs/`: About, Help, Quit confirmation (MessageBox)
- `ui/tray.rs`: System tray icon with context menu
- `storage/config.rs`: JSON persistence in %APPDATA%\Lumbus\

### Architecture Patterns

1. **Platform abstraction**: Entry point dispatches to platform-specific modules via cfg gates. Shared code (events, model) is platform-agnostic.

2. **FFI encapsulation**: 
   - macOS: All Cocoa/Carbon/CoreText FFI in `platform/macos/ffi/`
   - Windows: Uses `windows-rs` crate (type-safe bindings)

3. **State management**: 
   - macOS: State in CustomView instance variables (Objective-C runtime)
   - Windows: State in thread-local `RefCell<OverlayState>`

4. **Drawing strategy**:
   - macOS: `NSBezierPath` for circle, `CTFontCreatePathForGlyph` for letters
   - Windows: Direct2D `FillEllipse`/`DrawEllipse`, DirectWrite glyph outlines

5. **Persistence**: 
   - macOS: `NSUserDefaults`
   - Windows: JSON file in `%APPDATA%\Lumbus\config.json`

6. **Multi-display support**: 
   - macOS: One overlay window per `NSScreen`
   - Windows: Single layered window spanning virtual screen

## Global Hotkeys

| Action | macOS | Windows |
|--------|-------|---------|
| Toggle overlay | Ctrl+A | Ctrl+Shift+A |
| Open Settings | Ctrl+, | Ctrl+Shift+S |
| Show Help | Cmd+Shift+H | Ctrl+Shift+H |
| Quit | Ctrl+Shift+X | Ctrl+Shift+Q |

## Testing Notes

- All pure functions in `lib.rs` have corresponding tests in `tests/helpers.rs`
- Model validation tested in `tests/model_tests.rs`
- Event bus tested in `src/events/bus.rs` (unit tests)
- No integration/UI tests (platform UI testing is non-trivial)
- Total: 63 tests

### Manual Testing

- **macOS**: Test directly on development machine
- **Windows**: Test on Windows 11 ARM virtual machine (clone repo, `cargo run --release`)

## Platform Requirements

### macOS
- macOS 10.13+ (uses modern Cocoa/CoreGraphics APIs)
- Grant **Accessibility** and **Input Monitoring** permissions

### Windows
- Windows 10/11
- Visual Studio Build Tools (for MSVC linker when building)
- No special permissions required

## Known Constraints

- No CLI arguments; all configuration via GUI
- Settings persist automatically on change
- Overlay doesn't capture mouse events (click-through)

## Development Context

When modifying:
- **Cross-platform code** (`events/`, `model/`, `lib.rs`): Keep it free of FFI/platform dependencies
- **macOS code**: Preserve FFI patterns and Objective-C state management
- **Windows code**: Use `windows-rs` types, maintain layered window approach
- **Testing**: Test pure helpers thoroughly; UI/FFI changes require manual testing

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

# Build Universal Binary (arm64 + x86_64) for macOS
./scripts/build-universal.sh
```

## CI/CD and GitHub Actions

### Release Workflow (`.github/workflows/release.yml`)

The release workflow **triggers automatically** when a tag matching `v*` is pushed. It builds artifacts for all platforms and creates a GitHub Release.

**IMPORTANT:** Always review this workflow when modifying build scripts or adding new targets.

#### What it builds

| Job | Runner | Output |
|-----|--------|--------|
| `build-macos` | `macos-latest` | `Lumbus-X.Y.Z.dmg` (Universal Binary: arm64 + x86_64) |
| `build-windows-x64` | `windows-latest` | `lumbus-X.Y.Z-windows-x64.exe` |
| `build-windows-arm64` | `windows-latest` | `lumbus-X.Y.Z-windows-arm64.exe` |

#### Required Rust targets in CI

The workflow installs these targets as needed:
- **macOS:** `x86_64-apple-darwin` (for Universal Binary, since runner is arm64)
- **Windows ARM64:** `aarch64-pc-windows-msvc`

#### Workflow dependencies

```
build-macos ──────┐
build-windows-x64 ├──► create-release (uploads all artifacts)
build-windows-arm64 ──┘
```

If ANY build job fails, the release is NOT created.

#### Checklist when modifying build process

- [ ] Update `scripts/build-*.sh` as needed
- [ ] Update `.github/workflows/release.yml` to match
- [ ] Ensure required Rust targets are installed in workflow
- [ ] Test locally before pushing tag

### Creating a Release

```bash
# 1. Update version in Cargo.toml and resources/Info.plist
# 2. Commit changes
git add -A && git commit -m "chore: bump version to X.Y.Z"

# 3. Create and push tag (triggers workflow)
git tag vX.Y.Z && git push origin main --tags

# 4. Monitor workflow
gh run list --limit 1
gh run watch  # interactive monitoring
```

## Architecture Overview

The codebase follows a **platform-abstraction pattern** with conditional compilation (`#[cfg(target_os = "...")]`).

### Project Structure

```
src/
├── main.rs                     # Entry point with cfg gates (~30 lines)
├── macos_main.rs               # macOS app orchestrator (~185 lines)
├── windows_main.rs             # Windows app orchestrator (~285 lines)
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
    │   ├── input/              # Hotkeys (Carbon), mouse monitors
    │   ├── storage/            # NSUserDefaults persistence
    │   └── ui/                 # Overlay (view.rs), settings, dialogs, status bar
    └── windows/                # Windows implementation
        ├── app/                # State management (state.rs)
        ├── ffi/                # Win32 type definitions
        ├── input/              # Hotkeys, mouse hooks (hotkeys.rs)
        ├── storage/            # JSON config persistence
        └── ui/                 # Overlay (renderer.rs), settings, dialogs, tray
```

### Core Components

**`src/main.rs`** (~30 lines)
- Minimal entry point with platform cfg gates
- Initialises event bus, then calls `macos_main::run()` or `windows_main::run()`

**`src/macos_main.rs`** (~185 lines)
- App orchestrator: window creation, timer setup, hotkey installation
- CustomView logic extracted to `platform/macos/ui/overlay/view.rs`

**`src/windows_main.rs`** (~285 lines)
- App orchestrator: COM init, window creation, message loop
- State/rendering extracted to `platform/windows/app/` and `ui/overlay/`

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
- Total: 65 tests

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

#!/bin/bash
set -e

# Configuration
APP_NAME="Lumbus"
BUNDLE_NAME="Lumbus.app"
BINARY_NAME="lumbus"

# Paths
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/release"
APP_DIR="$PROJECT_ROOT/target/release/$BUNDLE_NAME"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

# Parse arguments
BUILD_TYPE="release"
BUILD_UNIVERSAL=false
SIGN_APP=false
SIGN_IDENTITY=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --debug)
            BUILD_TYPE="debug"
            BUILD_DIR="$PROJECT_ROOT/target/debug"
            APP_DIR="$PROJECT_ROOT/target/debug/$BUNDLE_NAME"
            CONTENTS_DIR="$APP_DIR/Contents"
            MACOS_DIR="$CONTENTS_DIR/MacOS"
            RESOURCES_DIR="$CONTENTS_DIR/Resources"
            shift
            ;;
        --universal)
            BUILD_UNIVERSAL=true
            BUILD_DIR="$PROJECT_ROOT/target/universal-apple-darwin/release"
            APP_DIR="$PROJECT_ROOT/target/universal-apple-darwin/release/$BUNDLE_NAME"
            CONTENTS_DIR="$APP_DIR/Contents"
            MACOS_DIR="$CONTENTS_DIR/MacOS"
            RESOURCES_DIR="$CONTENTS_DIR/Resources"
            shift
            ;;
        --sign)
            SIGN_APP=true
            SIGN_IDENTITY="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--debug] [--universal] [--sign \"Developer ID Application: NAME (TEAM)\"]"
            exit 1
            ;;
    esac
done

echo "=== Building $APP_NAME.app ==="
echo "Build type: $BUILD_TYPE"
echo "Universal: $BUILD_UNIVERSAL"

# Step 1: Build the Rust binary
echo ">>> Building Rust binary..."
if [ "$BUILD_UNIVERSAL" = true ]; then
    "$PROJECT_ROOT/scripts/build-universal.sh"
elif [ "$BUILD_TYPE" = "release" ]; then
    cargo build --release --manifest-path="$PROJECT_ROOT/Cargo.toml"
else
    cargo build --manifest-path="$PROJECT_ROOT/Cargo.toml"
fi

# Step 2: Verify binary exists
if [ ! -f "$BUILD_DIR/$BINARY_NAME" ]; then
    echo "Error: Binary not found at $BUILD_DIR/$BINARY_NAME"
    exit 1
fi

# Step 3: Create .app bundle structure
echo ">>> Creating app bundle structure..."
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# Step 4: Copy binary
echo ">>> Copying binary..."
cp "$BUILD_DIR/$BINARY_NAME" "$MACOS_DIR/"
chmod +x "$MACOS_DIR/$BINARY_NAME"

# Step 5: Copy Info.plist
echo ">>> Copying Info.plist..."
cp "$PROJECT_ROOT/resources/Info.plist" "$CONTENTS_DIR/"

# Step 6: Copy icon (if exists)
if [ -f "$PROJECT_ROOT/resources/icons/AppIcon.icns" ]; then
    echo ">>> Copying app icon..."
    cp "$PROJECT_ROOT/resources/icons/AppIcon.icns" "$RESOURCES_DIR/"
else
    echo "Warning: No icon found at resources/icons/AppIcon.icns"
fi

# Step 6b: Copy status bar icon
if [ -f "$PROJECT_ROOT/resources/icons/StatusBarIcon.png" ]; then
    echo ">>> Copying status bar icon..."
    cp "$PROJECT_ROOT/resources/icons/StatusBarIcon.png" "$RESOURCES_DIR/StatusBarIcon.png"
else
    echo "Warning: No status bar icon found at resources/icons/StatusBarIcon.png"
fi

# Step 6c: Copy Credits.rtf (for About panel)
if [ -f "$PROJECT_ROOT/resources/Credits.rtf" ]; then
    echo ">>> Copying Credits.rtf..."
    cp "$PROJECT_ROOT/resources/Credits.rtf" "$RESOURCES_DIR/"
fi

# Step 7: Create PkgInfo
echo ">>> Creating PkgInfo..."
echo -n "APPLmhlt" > "$CONTENTS_DIR/PkgInfo"

# Step 8: Code signing (optional)
if [ "$SIGN_APP" = true ]; then
    echo ">>> Code signing with identity: $SIGN_IDENTITY"
    codesign --force --deep --sign "$SIGN_IDENTITY" \
        --options runtime \
        --entitlements "$PROJECT_ROOT/resources/Lumbus.entitlements" \
        "$APP_DIR"

    echo ">>> Verifying signature..."
    codesign --verify --verbose "$APP_DIR"
fi

# Step 9: Display result
echo ""
echo "=== Build Complete ==="
echo "App bundle: $APP_DIR"
echo "Binary size: $(du -h "$MACOS_DIR/$BINARY_NAME" | cut -f1)"
echo ""

# Verify bundle structure
echo "Bundle structure:"
find "$APP_DIR" -type f | sed "s|$APP_DIR/||" | sort

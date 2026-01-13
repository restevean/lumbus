#!/bin/bash
#
# Build Lumbus.app and create DMG installer
#
# Usage: ./scripts/build-dmg.sh
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Building Lumbus DMG ===${NC}"

# Paths
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/release"
APP_NAME="Lumbus"
APP_BUNDLE="$BUILD_DIR/$APP_NAME.app"
DMG_NAME="Lumbus-$(grep '^version' "$PROJECT_ROOT/Cargo.toml" | cut -d'"' -f2)"
DMG_OUTPUT="$PROJECT_ROOT/dist/$DMG_NAME.dmg"

# Resources
ICON_FILE="$PROJECT_ROOT/resources/icons/AppIcon.icns"
STATUS_BAR_ICON="$PROJECT_ROOT/resources/icons/StatusBarIcon.png"
DMG_BACKGROUND="$PROJECT_ROOT/resources/dmg_background.png"
INFO_PLIST="$PROJECT_ROOT/resources/Info.plist"

# Create dist directory
mkdir -p "$PROJECT_ROOT/dist"

# Step 1: Build release
echo -e "${YELLOW}[1/4] Compiling release build...${NC}"
cargo build --release

# Step 2: Create .app bundle
echo -e "${YELLOW}[2/4] Creating app bundle...${NC}"
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# Copy binary
cp "$BUILD_DIR/lumbus" "$APP_BUNDLE/Contents/MacOS/$APP_NAME"

# Copy Info.plist
cp "$INFO_PLIST" "$APP_BUNDLE/Contents/Info.plist"

# Copy icons
cp "$ICON_FILE" "$APP_BUNDLE/Contents/Resources/AppIcon.icns"
cp "$STATUS_BAR_ICON" "$APP_BUNDLE/Contents/Resources/StatusBarIcon.png"

# Set executable permissions
chmod +x "$APP_BUNDLE/Contents/MacOS/$APP_NAME"

echo -e "${GREEN}   Created: $APP_BUNDLE${NC}"

# Step 3: Remove old DMG if exists
if [ -f "$DMG_OUTPUT" ]; then
    echo -e "${YELLOW}[3/4] Removing old DMG...${NC}"
    rm -f "$DMG_OUTPUT"
fi

# Step 4: Create DMG
echo -e "${YELLOW}[4/4] Creating DMG installer...${NC}"
create-dmg \
    --volname "$APP_NAME" \
    --volicon "$ICON_FILE" \
    --background "$DMG_BACKGROUND" \
    --window-pos 200 120 \
    --window-size 600 400 \
    --icon-size 80 \
    --icon "$APP_NAME.app" 160 160 \
    --hide-extension "$APP_NAME.app" \
    --app-drop-link 440 160 \
    "$DMG_OUTPUT" \
    "$APP_BUNDLE"

echo ""
echo -e "${GREEN}=== Build Complete ===${NC}"
echo -e "DMG created: ${YELLOW}$DMG_OUTPUT${NC}"
echo ""
echo "To test:"
echo "  open \"$DMG_OUTPUT\""

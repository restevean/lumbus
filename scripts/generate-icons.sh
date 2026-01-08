#!/bin/bash
set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE_IMAGE="${1:-$PROJECT_ROOT/resources/icons/source.png}"
ICONSET_DIR="$PROJECT_ROOT/resources/icons/AppIcon.iconset"
OUTPUT_ICNS="$PROJECT_ROOT/resources/icons/AppIcon.icns"

# Ensure source exists
if [ ! -f "$SOURCE_IMAGE" ]; then
    echo "Error: Source image not found at $SOURCE_IMAGE"
    echo ""
    echo "Usage: $0 [path-to-1024x1024-png]"
    echo ""
    echo "The source image should be at least 1024x1024 pixels."
    echo "Default location: resources/icons/source.png"
    exit 1
fi

# Check image dimensions
DIMENSIONS=$(sips -g pixelWidth -g pixelHeight "$SOURCE_IMAGE" 2>/dev/null | tail -2 | awk '{print $2}')
WIDTH=$(echo "$DIMENSIONS" | head -1)
HEIGHT=$(echo "$DIMENSIONS" | tail -1)

if [ "$WIDTH" -lt 1024 ] || [ "$HEIGHT" -lt 1024 ]; then
    echo "Warning: Source image is ${WIDTH}x${HEIGHT}. Recommended minimum is 1024x1024."
fi

echo "=== Generating App Icons ==="
echo "Source: $SOURCE_IMAGE (${WIDTH}x${HEIGHT})"

# Create iconset directory
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

# Generate all required sizes
echo ">>> Generating icon sizes..."
sizes=(16 32 128 256 512)
for size in "${sizes[@]}"; do
    echo "  - ${size}x${size}"
    sips -z $size $size "$SOURCE_IMAGE" --out "$ICONSET_DIR/icon_${size}x${size}.png" >/dev/null 2>&1

    # @2x variants (retina)
    double=$((size * 2))
    echo "  - ${size}x${size}@2x (${double}x${double})"
    sips -z $double $double "$SOURCE_IMAGE" --out "$ICONSET_DIR/icon_${size}x${size}@2x.png" >/dev/null 2>&1
done

# Generate .icns from iconset
echo ">>> Creating .icns file..."
iconutil -c icns "$ICONSET_DIR" -o "$OUTPUT_ICNS"

# Cleanup iconset directory (optional - uncomment to keep)
# rm -rf "$ICONSET_DIR"

echo ""
echo "=== Icon Generation Complete ==="
echo "Output: $OUTPUT_ICNS"
echo "Size: $(du -h "$OUTPUT_ICNS" | cut -f1)"

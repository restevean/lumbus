#!/usr/bin/env python3
"""
Darken areas under icons in DMG background to force white text labels.

The Finder determines icon label color based on the brightness of the
background area directly beneath each icon. This script adds dark
semi-transparent overlays where icons will be placed.
"""

from PIL import Image, ImageDraw

# DMG window configuration (must match build-dmg.sh)
WINDOW_WIDTH = 600
WINDOW_HEIGHT = 400

# Icon positions from build-dmg.sh (center coordinates)
APP_ICON_POS = (160, 160)
APPS_LINK_POS = (440, 160)

# Icon and label dimensions
ICON_SIZE = 80  # --icon-size in build-dmg.sh
LABEL_HEIGHT = 40  # Approximate height for text label below icon
PADDING = 20  # Extra padding around the area

def darken_background(input_path: str, output_path: str) -> None:
    """Add dark overlays under icon positions."""
    img = Image.open(input_path).convert("RGBA")

    # Verify dimensions match expected
    if img.size != (WINDOW_WIDTH, WINDOW_HEIGHT):
        print(f"Warning: Image size {img.size} doesn't match expected {WINDOW_WIDTH}x{WINDOW_HEIGHT}")

    # Create overlay layer
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)

    # Calculate area to darken for each icon (icon + label below it)
    for pos in [APP_ICON_POS, APPS_LINK_POS]:
        x, y = pos
        # Rectangle covering icon and label area
        # y coordinate in Finder is from top, icon centered at position
        left = x - ICON_SIZE // 2 - PADDING
        top = y - ICON_SIZE // 2 - PADDING
        right = x + ICON_SIZE // 2 + PADDING
        bottom = y + ICON_SIZE // 2 + LABEL_HEIGHT + PADDING

        # Draw light rectangle with rounded corners
        # Using light overlay for black text readability
        draw.rounded_rectangle(
            [left, top, right, bottom],
            radius=15,
            fill=(255, 255, 255, 80)
        )

    # Composite overlay onto original
    result = Image.alpha_composite(img, overlay)

    # Save as PNG
    result.save(output_path, "PNG")
    print(f"Saved darkened background to: {output_path}")

if __name__ == "__main__":
    import sys

    if len(sys.argv) >= 3:
        input_file = sys.argv[1]
        output_file = sys.argv[2]
    else:
        # Default paths
        input_file = "resources/dmg_background.png"
        output_file = "resources/dmg_background.png"

    darken_background(input_file, output_file)

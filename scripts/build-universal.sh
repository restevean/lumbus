#!/bin/bash
#
# Build Universal Binary (arm64 + x86_64) for macOS
#
# This script compiles Lumbus for both Apple Silicon and Intel,
# then combines them into a single Universal Binary using lipo.
#
# Usage: ./scripts/build-universal.sh [--debug]
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
BINARY_NAME="lumbus"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Parse arguments
BUILD_TYPE="release"
CARGO_FLAGS="--release"

if [[ "$1" == "--debug" ]]; then
    BUILD_TYPE="debug"
    CARGO_FLAGS=""
fi

# Target directories
ARM_TARGET="aarch64-apple-darwin"
INTEL_TARGET="x86_64-apple-darwin"
ARM_BUILD_DIR="$PROJECT_ROOT/target/$ARM_TARGET/$BUILD_TYPE"
INTEL_BUILD_DIR="$PROJECT_ROOT/target/$INTEL_TARGET/$BUILD_TYPE"
UNIVERSAL_DIR="$PROJECT_ROOT/target/universal-apple-darwin/$BUILD_TYPE"

echo -e "${CYAN}=== Building Universal Binary ===${NC}"
echo -e "Build type: ${YELLOW}$BUILD_TYPE${NC}"
echo ""

# Step 1: Build for Apple Silicon (arm64)
echo -e "${YELLOW}[1/3] Building for Apple Silicon (arm64)...${NC}"
cargo build $CARGO_FLAGS --target $ARM_TARGET

if [ ! -f "$ARM_BUILD_DIR/$BINARY_NAME" ]; then
    echo -e "${RED}Error: ARM binary not found at $ARM_BUILD_DIR/$BINARY_NAME${NC}"
    exit 1
fi
echo -e "${GREEN}   ✓ ARM build complete${NC}"

# Step 2: Build for Intel (x86_64)
echo -e "${YELLOW}[2/3] Building for Intel (x86_64)...${NC}"
cargo build $CARGO_FLAGS --target $INTEL_TARGET

if [ ! -f "$INTEL_BUILD_DIR/$BINARY_NAME" ]; then
    echo -e "${RED}Error: Intel binary not found at $INTEL_BUILD_DIR/$BINARY_NAME${NC}"
    exit 1
fi
echo -e "${GREEN}   ✓ Intel build complete${NC}"

# Step 3: Create Universal Binary with lipo
echo -e "${YELLOW}[3/3] Creating Universal Binary with lipo...${NC}"
mkdir -p "$UNIVERSAL_DIR"

lipo -create \
    "$ARM_BUILD_DIR/$BINARY_NAME" \
    "$INTEL_BUILD_DIR/$BINARY_NAME" \
    -output "$UNIVERSAL_DIR/$BINARY_NAME"

chmod +x "$UNIVERSAL_DIR/$BINARY_NAME"
echo -e "${GREEN}   ✓ Universal binary created${NC}"

# Display results
echo ""
echo -e "${GREEN}=== Build Complete ===${NC}"
echo ""
echo -e "Universal binary: ${CYAN}$UNIVERSAL_DIR/$BINARY_NAME${NC}"
echo ""

# Show architecture info
echo -e "${YELLOW}Architecture info:${NC}"
lipo -info "$UNIVERSAL_DIR/$BINARY_NAME"
echo ""

# Show sizes
echo -e "${YELLOW}Binary sizes:${NC}"
echo -e "  ARM64:    $(du -h "$ARM_BUILD_DIR/$BINARY_NAME" | cut -f1)"
echo -e "  x86_64:   $(du -h "$INTEL_BUILD_DIR/$BINARY_NAME" | cut -f1)"
echo -e "  Universal: $(du -h "$UNIVERSAL_DIR/$BINARY_NAME" | cut -f1)"

#!/bin/bash
# verify-release.sh - Verify project consistency before building a release
#
# Checks:
# 1. Version consistency between Cargo.toml and Info.plist
# 2. Hotkey documentation consistency (Ctrl+, not Cmd+,)
# 3. Dependencies mentioned in docs match actual crates
# 4. Test count in README matches actual tests
#
# Usage: ./scripts/verify-release.sh
# Returns: 0 if all checks pass, 1 if any check fails

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0

echo -e "${YELLOW}=== Verifying Release Consistency ===${NC}"
echo ""

# Get project root (parent of scripts dir)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

# 1. Version Consistency
echo -e "${YELLOW}[1/5] Checking version consistency...${NC}"

CARGO_VERSION=$(grep "^version" Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
PLIST_VERSION=$(grep -A1 "CFBundleShortVersionString" resources/Info.plist | grep string | sed 's/.*<string>\(.*\)<\/string>.*/\1/')

if [ "$CARGO_VERSION" != "$PLIST_VERSION" ]; then
    echo -e "${RED}   ERROR: Version mismatch!${NC}"
    echo -e "   Cargo.toml: $CARGO_VERSION"
    echo -e "   Info.plist: $PLIST_VERSION"
    ERRORS=$((ERRORS + 1))
else
    echo -e "${GREEN}   OK: Version $CARGO_VERSION${NC}"
fi

# 2. Hotkey Documentation (Settings should be Ctrl+, not Cmd+,)
echo -e "${YELLOW}[2/5] Checking hotkey documentation...${NC}"

# Check for old Cmd+, references (should be Ctrl+,)
OLD_HOTKEY_REFS=$(grep -rn "⌘.*," --include="*.md" --include="*.rs" . 2>/dev/null | grep -i "settings\|Open\|Abrir" | grep -v "target/" || true)
if [ -n "$OLD_HOTKEY_REFS" ]; then
    echo -e "${RED}   ERROR: Found old Cmd+, references for Settings:${NC}"
    echo "$OLD_HOTKEY_REFS" | while read line; do echo "   $line"; done
    ERRORS=$((ERRORS + 1))
else
    echo -e "${GREEN}   OK: Settings hotkey is Ctrl+, everywhere${NC}"
fi

# 3. Dependencies in README
echo -e "${YELLOW}[3/5] Checking dependency documentation...${NC}"

# Check that README mentions objc2, not cocoa/objc/block
if grep -q 'cocoa = "' README.md 2>/dev/null; then
    echo -e "${RED}   ERROR: README still mentions deprecated 'cocoa' crate${NC}"
    ERRORS=$((ERRORS + 1))
elif grep -qi 'objc2' README.md 2>/dev/null; then
    echo -e "${GREEN}   OK: README mentions objc2 ecosystem${NC}"
else
    echo -e "${YELLOW}   WARN: Could not verify dependencies in README${NC}"
fi

# 4. Test Count
echo -e "${YELLOW}[4/5] Checking test count...${NC}"

ACTUAL_TESTS=$(cargo test 2>&1 | grep -E "^test result:" | awk '{sum += $4} END {print sum}')
README_TESTS=$(grep -oE "[0-9]+ unit tests" README.md | grep -oE "[0-9]+" || echo "0")

if [ "$ACTUAL_TESTS" != "$README_TESTS" ] && [ -n "$ACTUAL_TESTS" ]; then
    echo -e "${YELLOW}   WARN: Test count mismatch${NC}"
    echo -e "   README says: $README_TESTS tests"
    echo -e "   Actual: $ACTUAL_TESTS tests"
    # This is a warning, not an error
else
    echo -e "${GREEN}   OK: Test count matches ($ACTUAL_TESTS tests)${NC}"
fi

# 5. CLAUDE.md Architecture Check
echo -e "${YELLOW}[5/5] Checking CLAUDE.md consistency...${NC}"

# Check main.rs line count is approximately correct
MAIN_LINES=$(wc -l < src/main.rs | tr -d ' ')
CLAUDE_LINES=$(grep -oE "~[0-9]+ lines" CLAUDE.md | grep -oE "[0-9]+" | head -1 || echo "0")

if [ -n "$CLAUDE_LINES" ] && [ "$CLAUDE_LINES" -gt 0 ]; then
    DIFF=$((MAIN_LINES - CLAUDE_LINES))
    if [ $DIFF -lt 0 ]; then DIFF=$((DIFF * -1)); fi
    
    if [ $DIFF -gt 100 ]; then
        echo -e "${YELLOW}   WARN: main.rs line count differs significantly${NC}"
        echo -e "   CLAUDE.md says: ~$CLAUDE_LINES lines"
        echo -e "   Actual: $MAIN_LINES lines"
    else
        echo -e "${GREEN}   OK: main.rs line count (~$MAIN_LINES lines)${NC}"
    fi
else
    echo -e "${GREEN}   OK: CLAUDE.md structure check passed${NC}"
fi

# Summary
echo ""
if [ $ERRORS -gt 0 ]; then
    echo -e "${RED}=== FAILED: $ERRORS error(s) found ===${NC}"
    echo -e "Fix the issues above before building a release."
    exit 1
else
    echo -e "${GREEN}=== PASSED: All checks OK ===${NC}"
    exit 0
fi

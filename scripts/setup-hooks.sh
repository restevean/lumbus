#!/bin/bash
# setup-hooks.sh - Install git hooks for this project
#
# Usage: ./scripts/setup-hooks.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
HOOKS_SRC="$SCRIPT_DIR/hooks"
HOOKS_DST="$PROJECT_ROOT/.git/hooks"

echo "Installing git hooks..."

if [ ! -d "$HOOKS_SRC" ]; then
    echo "Error: $HOOKS_SRC not found"
    exit 1
fi

for hook in "$HOOKS_SRC"/*; do
    if [ -f "$hook" ]; then
        hook_name=$(basename "$hook")
        cp "$hook" "$HOOKS_DST/$hook_name"
        chmod +x "$HOOKS_DST/$hook_name"
        echo "  Installed: $hook_name"
    fi
done

echo "Done. Git hooks installed."

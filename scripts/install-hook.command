#!/bin/bash
# install-hook.command — Install polyref enforcement hook for Claude Code (macOS)
# Double-click this file in Finder to install. Merges into existing hooks.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "============================================"
echo "  Polyref Enforcement Hook Installer"
echo "============================================"
echo ""

# Check for polyref binary
POLYREF_BIN=""
if [ -f "$PROJECT_ROOT/target/release/polyref" ]; then
    POLYREF_BIN="$PROJECT_ROOT/target/release/polyref"
elif [ -f "$PROJECT_ROOT/target/debug/polyref" ]; then
    POLYREF_BIN="$PROJECT_ROOT/target/debug/polyref"
fi

if [ -n "$POLYREF_BIN" ]; then
    echo "Found polyref at: $POLYREF_BIN"
else
    echo "WARNING: polyref binary not found. Building..."
    cd "$PROJECT_ROOT"
    cargo build --release
    POLYREF_BIN="$PROJECT_ROOT/target/release/polyref"
fi

# Global Claude settings
SETTINGS_FILE="$HOME/.claude/settings.json"
HOOK_CMD="bash \"$PROJECT_ROOT/scripts/enforce-pipeline.sh\""

echo ""
echo "Merging enforce hook into: $SETTINGS_FILE"
python3 "$PROJECT_ROOT/scripts/merge_hook.py" "$SETTINGS_FILE" "$HOOK_CMD"

echo ""
echo "SUCCESS: Enforce pipeline hook installed."
echo "Existing hooks were preserved."
echo ""
read -p "Press Enter to close..."

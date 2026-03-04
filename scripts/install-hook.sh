#!/usr/bin/env bash
# install-hook.sh — Install polyref enforcement hooks for Claude Code
#
# Usage:
#   bash scripts/install-hook.sh [--audit-only]
#
# Options:
#   --audit-only  Use non-blocking audit mode instead of full enforcement

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CLAUDE_DIR="$PROJECT_ROOT/.claude"

# Check polyref is available
if ! command -v polyref &>/dev/null; then
    echo "WARNING: polyref not found on PATH" >&2
    echo "Install it or add target/release to PATH" >&2
fi

# Determine which settings to use
if [ "${1:-}" = "--audit-only" ]; then
    SOURCE_FILE="$CLAUDE_DIR/settings.example.json"
    echo "Installing audit-only (non-blocking) hooks..."
else
    SOURCE_FILE="$CLAUDE_DIR/settings.json"
    echo "Installing full enforcement hooks..."
fi

# Create .claude directory if needed
mkdir -p "$CLAUDE_DIR"

# Copy settings file
if [ -f "$SOURCE_FILE" ]; then
    cp "$SOURCE_FILE" "$CLAUDE_DIR/settings.json"
    echo "Installed: $CLAUDE_DIR/settings.json"
else
    echo "ERROR: Source settings file not found: $SOURCE_FILE" >&2
    exit 1
fi

echo "Done. Polyref hooks are now active."

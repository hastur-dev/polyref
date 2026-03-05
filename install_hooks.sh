#!/usr/bin/env bash
set -euo pipefail

DAEMON_BIN="./target/release/crate-ref-daemon"
HOOK_BIN="./target/release/crate-ref-hook"

echo "Building release binaries..."
cargo build --release --workspace

echo "Installing Claude Code hooks..."

# Claude Code hooks config: ~/.config/claude/hooks.json (or local .claude/hooks.json)
HOOKS_DIR="${CLAUDE_HOOKS_DIR:-.claude}"
mkdir -p "$HOOKS_DIR"

cat > "$HOOKS_DIR/hooks.json" <<EOF
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit|MultiEdit",
        "hooks": [
          {
            "type": "command",
            "command": "${HOOK_BIN}"
          }
        ]
      }
    ]
  }
}
EOF

echo "Hooks installed to $HOOKS_DIR/hooks.json"
echo "Start daemon manually with: $DAEMON_BIN &"
echo "Or let the hook shim auto-start it on first use."

#!/usr/bin/env bash
# post-write-audit.sh — Non-blocking post-write audit
# Runs cargo test after writes and logs failures to .polyref-audit.jsonl.
# Always exits 0 (non-blocking).

set -uo pipefail

PROJECT_ROOT="${POLYREF_PROJECT_ROOT:-.}"
AUDIT_LOG="${PROJECT_ROOT}/.polyref-audit.jsonl"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    exit 0
fi

OUTPUT="$(cargo test --manifest-path "$PROJECT_ROOT/Cargo.toml" --quiet 2>&1)" || true
EXIT_CODE=$?

if [ "$EXIT_CODE" -ne 0 ]; then
    # Escape JSON-unsafe characters
    ESCAPED_OUTPUT="$(echo "$OUTPUT" | head -20 | sed 's/\\/\\\\/g; s/"/\\"/g' | tr '\n' '|')"
    echo "{\"timestamp\":\"$TIMESTAMP\",\"event\":\"test_failure\",\"exit_code\":$EXIT_CODE,\"output\":\"$ESCAPED_OUTPUT\"}" \
        >> "$AUDIT_LOG"
fi

exit 0

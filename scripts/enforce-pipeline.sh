#!/usr/bin/env bash
# enforce-pipeline.sh — Multi-layer enforcement gate for polyref
# Reads proposed source from stdin, validates through multiple layers.
# Exits 1 on ANY layer failure; reports which layer failed.
#
# Environment variables:
#   POLYREF_REFS_DIR     — path to reference files (default: ./refs)
#   POLYREF_LANG         — language hint: rust|python|typescript (default: auto)
#   POLYREF_PROJECT_ROOT — project root (default: .)

set -euo pipefail

REFS_DIR="${POLYREF_REFS_DIR:-./refs}"
LANG_HINT="${POLYREF_LANG:-auto}"
PROJECT_ROOT="${POLYREF_PROJECT_ROOT:-.}"

# Read source from stdin into a temp file
TMPFILE="$(mktemp --suffix=".rs")"
trap 'rm -f "$TMPFILE"' EXIT

cat > "$TMPFILE"

if [ ! -s "$TMPFILE" ]; then
    echo "ERROR: No input provided on stdin" >&2
    exit 1
fi

FAILED_LAYER=""

# Layer 1: polyref enforce
echo "--- Layer 1: polyref enforce ---"
if ! polyref enforce --from-stdin --enforce --lang "$LANG_HINT" \
    --refs "$REFS_DIR" --output-format json < "$TMPFILE"; then
    FAILED_LAYER="polyref"
fi

# Layer 2: cargo check (Rust only)
if [ "$LANG_HINT" = "rust" ] || [ "$LANG_HINT" = "auto" ]; then
    echo "--- Layer 2: cargo check ---"
    if [ -f "$PROJECT_ROOT/Cargo.toml" ]; then
        if ! cargo check --manifest-path "$PROJECT_ROOT/Cargo.toml" --quiet 2>/dev/null; then
            FAILED_LAYER="${FAILED_LAYER:+$FAILED_LAYER, }cargo-check"
        fi
    fi
fi

# Layer 3: clippy (Rust only)
if [ "$LANG_HINT" = "rust" ] || [ "$LANG_HINT" = "auto" ]; then
    echo "--- Layer 3: clippy ---"
    if [ -f "$PROJECT_ROOT/Cargo.toml" ]; then
        if ! cargo clippy --manifest-path "$PROJECT_ROOT/Cargo.toml" --quiet -- -D warnings 2>/dev/null; then
            FAILED_LAYER="${FAILED_LAYER:+$FAILED_LAYER, }clippy"
        fi
    fi
fi

# Layer 4: cargo audit (if available)
if command -v cargo-audit &>/dev/null; then
    echo "--- Layer 4: cargo audit ---"
    if [ -f "$PROJECT_ROOT/Cargo.toml" ]; then
        if ! cargo audit --quiet 2>/dev/null; then
            FAILED_LAYER="${FAILED_LAYER:+$FAILED_LAYER, }cargo-audit"
        fi
    fi
fi

# Report result
if [ -n "$FAILED_LAYER" ]; then
    echo "BLOCKED: Failed layer(s): $FAILED_LAYER" >&2
    exit 1
fi

echo "APPROVED: All layers passed"
exit 0

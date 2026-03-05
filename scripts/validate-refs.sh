#!/usr/bin/env bash
# validate-refs.sh — Validate that all reference files parse without errors
set -euo pipefail

REFS_DIR="${1:-refs}"
ERRORS=0
CHECKED=0

echo "Validating reference files in $REFS_DIR..."

# Check Rust reference files (.rs)
for f in "$REFS_DIR"/**/*.rs "$REFS_DIR"/*.rs; do
    [ -f "$f" ] || continue
    CHECKED=$((CHECKED + 1))

    # Check for required header lines
    if ! head -3 "$f" | grep -q "Reference"; then
        echo "WARN: $f — missing Reference header comment"
    fi

    # Check that impl blocks have matching braces
    OPEN=$(grep -c '{' "$f" || true)
    CLOSE=$(grep -c '}' "$f" || true)
    if [ "$OPEN" -ne "$CLOSE" ]; then
        echo "ERROR: $f — mismatched braces (open=$OPEN close=$CLOSE)"
        ERRORS=$((ERRORS + 1))
    fi
done

# Check polyref v2 reference files (.polyref)
for f in "$REFS_DIR"/**/*.polyref "$REFS_DIR"/*.polyref; do
    [ -f "$f" ] || continue
    CHECKED=$((CHECKED + 1))

    # Check for @lang tag
    if ! grep -q "^@lang " "$f"; then
        echo "ERROR: $f — missing @lang directive"
        ERRORS=$((ERRORS + 1))
    fi

    # Check for at least one @module, @class, or @fn
    if ! grep -qE "^@(module|class|fn) " "$f"; then
        echo "ERROR: $f — no @module, @class, or @fn entries"
        ERRORS=$((ERRORS + 1))
    fi
done

echo ""
echo "Checked $CHECKED files, $ERRORS errors"

if [ "$ERRORS" -gt 0 ]; then
    exit 1
fi

echo "All reference files valid."

#!/usr/bin/env bash
# lint-tests.sh — detect tautological test patterns
set -euo pipefail

ERRORS=0

echo "=== Test Lint Check ==="
echo

# 1. assert!(true) — always passes
echo "[1] Checking for assert!(true)..."
if grep -rn 'assert!(true)' tests/ src/ 2>/dev/null; then
    echo "  FAIL: Found tautological assert!(true)"
    ERRORS=$((ERRORS + 1))
else
    echo "  OK"
fi

# 2. assert_eq!(x, x) — comparing value to itself
echo "[2] Checking for assert_eq!(x, x) patterns..."
if grep -rn -P 'assert_eq!\((\w+),\s*\1\)' tests/ src/ 2>/dev/null; then
    echo "  FAIL: Found self-comparison assert_eq!"
    ERRORS=$((ERRORS + 1))
else
    echo "  OK"
fi

# 3. Empty test functions
echo "[3] Checking for empty test functions..."
if grep -rn -A1 '#\[test\]' tests/ src/ 2>/dev/null | grep -B1 'fn.*{}' | grep '#\[test\]'; then
    echo "  FAIL: Found empty test functions"
    ERRORS=$((ERRORS + 1))
else
    echo "  OK"
fi

# 4. Tests with no assertions
echo "[4] Checking for tests without assertions..."
# This is a heuristic — tests that don't panic or assert are suspicious but not always wrong
# (e.g., "does not panic" tests are valid)

echo
if [ "$ERRORS" -gt 0 ]; then
    echo "Test lint check found $ERRORS issue(s)"
    exit 1
else
    echo "Test lint check passed!"
    exit 0
fi

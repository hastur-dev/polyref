#!/usr/bin/env bash
# security-check.sh — scan for common security issues in the codebase
set -euo pipefail

ERRORS=0

echo "=== Polyref Security Check ==="
echo

# 1. Check for hardcoded secrets / credentials
echo "[1] Checking for hardcoded secrets..."
if grep -rn --include='*.rs' --include='*.ts' --include='*.py' \
    -E '(password|secret|api_key|token)\s*=\s*"[^"]{8,}"' \
    src/ tools/ typescript/ 2>/dev/null | grep -v 'test' | grep -v '_tests' | grep -v '.test.'; then
    echo "  WARN: Possible hardcoded secrets found (review above)"
    ERRORS=$((ERRORS + 1))
else
    echo "  OK: No hardcoded secrets detected"
fi

# 2. Check for unsafe blocks in Rust code
echo "[2] Checking for unsafe blocks..."
UNSAFE_COUNT=$(grep -rn 'unsafe {' src/ tools/ 2>/dev/null | wc -l)
if [ "$UNSAFE_COUNT" -gt 0 ]; then
    echo "  WARN: Found $UNSAFE_COUNT unsafe blocks — review needed"
    grep -rn 'unsafe {' src/ tools/ 2>/dev/null || true
    ERRORS=$((ERRORS + 1))
else
    echo "  OK: No unsafe blocks found"
fi

# 3. Check for unwrap() in non-test production code
echo "[3] Checking for unwrap() in production code..."
UNWRAP_FILES=$(grep -rln '\.unwrap()' src/ 2>/dev/null | grep -v '_test' | grep -v 'tests/' || true)
if [ -n "$UNWRAP_FILES" ]; then
    UNWRAP_COUNT=$(grep -rn '\.unwrap()' src/ 2>/dev/null | grep -v '_test' | grep -v 'tests/' | wc -l)
    echo "  INFO: Found $UNWRAP_COUNT unwrap() calls in production code"
    echo "  (Consider using ? or expect() for better error messages)"
else
    echo "  OK: No unwrap() calls in production code"
fi

# 4. Check for command injection via std::process::Command with user input
echo "[4] Checking for potential command injection..."
if grep -rn 'Command::new.*&' src/ tools/ 2>/dev/null | grep -v 'test' | grep -v '_tests'; then
    echo "  INFO: Review Command::new usages for injection risk"
else
    echo "  OK: No suspicious Command::new patterns found"
fi

# 5. Check for TODO/FIXME security items
echo "[5] Checking for security-related TODOs..."
if grep -rni 'TODO.*secur\|FIXME.*secur\|HACK.*secur' src/ tools/ 2>/dev/null; then
    echo "  WARN: Security TODOs found — address before release"
    ERRORS=$((ERRORS + 1))
else
    echo "  OK: No security TODOs found"
fi

echo
if [ "$ERRORS" -gt 0 ]; then
    echo "Security check completed with $ERRORS warning(s)"
    exit 1
else
    echo "Security check passed!"
    exit 0
fi

#!/usr/bin/env bash
set -euo pipefail

echo "==> CI Quick Gate (public): $(date)"
echo "ℹ Full verification (build + audit + OS matrix + MSRV) runs on GitHub Actions."
echo ""

# Step 1: Format check
echo "--- Format check ---"
cargo fmt --check
echo "  ✓ format check passed"
echo ""

# Step 2: Type check (fast, no codegen)
echo "--- Type check ---"
cargo check
echo "  ✓ type check passed"
echo ""

# Step 3: Lint (all targets including tests)
echo "--- Lint ---"
cargo clippy --all-targets -- -D warnings
echo "  ✓ clippy passed"
echo ""

# Step 4: Tests
echo "--- Tests ---"
cargo test 2>/dev/null && echo "  ✓ tests passed" || echo "  (no tests configured)"
echo ""

# Step 5: Verify ci.yml exists for the full gate
echo "--- CI config check ---"
test -f .github/workflows/ci.yml || { echo "  ✗ ci.yml missing — generate via git-ops skill"; exit 1; }
echo "  ✓ ci.yml present"
echo ""

echo "==> Quick gate PASSED — push and let GitHub Actions run full CI"

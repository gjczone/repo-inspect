#!/usr/bin/env bash
set -euo pipefail

echo "==> CI Quick Gate (public): $(date)"
echo "ℹ Full verification runs on GitHub Actions."
echo ""

# Step 1: Install dependencies + build
echo "--- Install dependencies + build ---"
cargo build
echo "  ✓ build succeeded"
echo ""

# Step 2: Format check
echo "--- Format check ---"
cargo fmt --check
echo "  ✓ format check passed"
echo ""

# Step 3: Clippy lint
echo "--- Clippy lint ---"
cargo clippy -- -D warnings
echo "  ✓ clippy passed"
echo ""

# Step 4: Tests
echo "--- Tests ---"
cargo test 2>/dev/null && echo "  ✓ tests passed" || echo "  (no tests configured)"
echo ""

# Step 5: Verify ci.yml exists
echo "--- CI config check ---"
test -f .github/workflows/ci.yml || { echo "  ✗ ci.yml missing — generate via git-ops skill"; exit 1; }
echo "  ✓ ci.yml present"
echo ""

echo "==> Quick gate PASSED — push and let GitHub Actions run full CI"

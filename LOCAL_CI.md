# LOCAL_CI.md — repo-inspect (fast pre-push gate)

Full CI runs on GitHub Actions. This is the fast pre-push checklist. See `rules/LOCAL_CI.md` for the detailed version with per-step pass/fix criteria.

## Quick Run (all steps)

```bash
cargo fmt --check && \
cargo clippy -- -D warnings && \
cargo build --release && \
cargo test && \
./target/release/repo-inspect --repo . find-how "search" --depth 1 && \
echo "ALL CHECKS PASSED"
```

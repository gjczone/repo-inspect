# OPS.md — repo-inspect

Release and operational procedures. See root `OPS.md` for the full 7-phase release workflow.

## Quick Reference

| Operation | Command |
|-----------|---------|
| Build | `cargo build --release` |
| Test | `cargo test` |
| Bundle | `cp target/release/repo-inspect skills/repo-inspect/scripts/` |
| Smoke | `./target/release/repo-inspect --repo . find-how "search" --depth 1` |
| Version | `grep '^version' Cargo.toml` |
| Release | See root `OPS.md` Phase 4 |

## Environment

- **Rust**: 1.85+ (2024 edition)
- **No external services**, no ports, no env vars required (except `GITHUB_TOKEN` for remote mode)
- **Clean reset**: `cargo clean && cargo build`

## Build Artifacts

| Artifact | Location | Size Target |
|----------|----------|-------------|
| Release binary | `target/release/repo-inspect` | < 6 MB |
| Bundled binary | `skills/repo-inspect/scripts/repo-inspect` | Same as release |
| Cargo lock | `Cargo.lock` | Auto-generated |

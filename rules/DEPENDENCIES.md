# Dependency Management Rules — repo-inspect

## Dependency Policy

- **NEVER** add a new dependency without explicit justification in the PR body.
- **NEVER** introduce async runtime (tokio/async-std) — the project is synchronous.
- **NEVER** add a dependency that pulls in 50+ transitive crates without strong justification.

## Current Dependencies

| Crate | Version | Purpose | Risk Level |
|-------|---------|---------|------------|
| `clap` | 4 | CLI argument parsing | Low |
| `ignore` | 0.4 | `.gitignore`-aware file traversal | Low |
| `regex` | 1 | Pattern matching | Low |
| `serde` / `serde_json` | 1 | Serialization | Low |
| `walkdir` | 2 | Directory walking | Low |
| `anyhow` | 1 | Application error handling | Low |
| `thiserror` | 2 | Library error types | Low |
| `log` / `env_logger` | 0.4 / 0.11 | Logging | Low |
| `minreq` | 2 | Sync HTTP client | Medium |
| `tree-sitter` + 4 grammars | Various | AST parsing | Medium |

## Adding a Dependency

1. Justify in PR: why this crate, why not an alternative, what problem it solves
2. Verify: license compatibility, maintenance status, crate size
3. Check `Cargo.lock` diff for transitive dependency blast radius
4. Verify `cargo build --release` binary size delta (< 1MB)
5. Update `Cargo.toml` → `cargo update` → verify no breakage

## Updating Dependencies

```bash
# Check for outdated
cargo update --dry-run

# Update specific
cargo update -p <crate>

# Verify after update
cargo build --release && cargo test
```

## Removing Dependencies

1. Remove from `Cargo.toml`
2. Delete all `use` imports and code paths
3. `cargo build` — confirm no compilation errors
4. Remove from this file's dependency table

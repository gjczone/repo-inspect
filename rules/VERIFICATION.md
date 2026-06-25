# Verification Rules — repo-inspect

All rules come from evidence: `LOCAL_CI.md`, `.github/workflows/ci.yml`, `Cargo.toml`.

## Verification Before Completion

Before marking ANY task as complete, ALL of these must pass:

- [ ] `cargo fmt --check` passes — zero diff
- [ ] `cargo clippy -- -D warnings` passes — zero warnings
- [ ] `cargo build --release` succeeds — exit 0
- [ ] `cargo test` passes — 0 failures, 0 ignored
- [ ] Binary updated: `cp target/release/repo-inspect skills/repo-inspect/scripts/`
- [ ] Manual smoke test: `./skills/repo-inspect/scripts/repo-inspect --repo . find-how "search" --depth 1` exits 0
- [ ] `skills/repo-inspect/references/commands.md` matches current CLI
- [ ] `git status --porcelain` returns empty

## Pre-Commit Checklist

```bash
# 1. Format
cargo fmt --check

# 2. Lint
cargo clippy -- -D warnings

# 3. Build
cargo build --release

# 4. Test
cargo test
```

All four must pass. If any fails → fix → re-run ALL four. No skipping.

## Pre-Push Checklist

Same as pre-commit, plus:

```bash
# 5. Smoke test (local mode)
./target/release/repo-inspect --repo . find-how "test" --depth 1

# 6. Binary size check (< 6 MB)
ls -lh target/release/repo-inspect

# 7. Update bundled binary
cp target/release/repo-inspect skills/repo-inspect/scripts/

# 8. Docs sync
# Verify skills/repo-inspect/references/commands.md matches src/cli.rs
```

## CI Verification

- **Local CI** (`LOCAL_CI.md`): Fast pre-push gate — format + clippy + build + test + smoke (< 60 seconds)
- **GitHub Actions** (`.github/workflows/ci.yml`): Comprehensive — same as LOCAL_CI.md, authoritative

For public repos: lightweight `LOCAL_CI.md` is the fast gate; GitHub Actions is the comprehensive authority.
For private repos: FULL `LOCAL_CI.md` is the sole authority.

## Remote Mode Verification

When remote code is changed:

```bash
# Cache hit test
./target/release/repo-inspect --repo gjczone/repo-inspect find-how "search" --depth 1

# Fresh fetch test
rm -rf ~/.cache/repo-inspect/remote/gjczone-repo-inspect
./target/release/repo-inspect --repo gjczone/repo-inspect find-how "search" --depth 1

# --refresh test
./target/release/repo-inspect --repo gjczone/repo-inspect find-how "search" --refresh

# All subcommands (remote)
./target/release/repo-inspect --repo gjczone/repo-inspect entries
./target/release/repo-inspect --repo gjczone/repo-inspect trace "search"
```

## Verification Anti-Patterns

| Anti-Pattern | Detection | Fix |
|--------------|-----------|-----|
| Skipping `cargo fmt` | Format diff in PR | Run `cargo fmt` |
| Ignoring clippy warnings | `cargo clippy` exits non-zero | Fix every warning |
| Skipping test run | `cargo test` not run | Run full suite |
| Stale binary in skill | Binary not updated after build | `cp target/release/... skills/...` |
| Stale docs | `commands.md` doesn't match CLI | Update docs |
| Dirty git | Uncommitted changes at completion | Commit or clean |

# LOCAL_CI.md — repo-inspect

Full CI runs on GitHub Actions — this is the fast pre-push gate. Run ALL steps before every push. Failing any = broken commit.

## 1. Format Check

```bash
cargo fmt --check
```

**Pass**: no diff output, exit 0.
**Fix**: `cargo fmt` then re-check.

## 2. Lint (Strict)

```bash
cargo clippy -- -D warnings
```

**Pass**: exit 0, zero warnings.
**Fix**: address every warning — treat warnings as compile errors.

## 3. Build (Release)

```bash
cargo build --release
```

**Pass**: exit 0, binary at `target/release/repo-inspect`.
**Fix**: read compiler errors, fix, rebuild.

## 4. Test

```bash
cargo test
```

**Pass**: exit 0, all tests pass, 0 failures, 0 ignored.
**Fix**: read test output, fix code or test, re-run.

## 5. Smoke Test

```bash
./target/release/repo-inspect --repo . find-how "search" --depth 1
```

**Pass**: exit 0, file created in `.inspect/`.
**Fix**: check stderr for error chain.

## 6. Binary Size Check

```bash
ls -lh target/release/repo-inspect
```

**Pass**: < 6 MB.
**Fix**: investigate bloat via `cargo bloat --release`.

## 7. Bundle Update

```bash
cp target/release/repo-inspect skills/repo-inspect/scripts/
```

**Pass**: bundled binary matches release binary.

## Quick Run (all steps)

```bash
cargo fmt --check && \
cargo clippy -- -D warnings && \
cargo build --release && \
cargo test && \
./target/release/repo-inspect --repo . find-how "search" --depth 1 && \
echo "ALL CHECKS PASSED"
```

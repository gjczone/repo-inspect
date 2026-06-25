# LOCAL_CI.md — repo-inspect

Full CI runs on GitHub Actions — this is the fast pre-push gate. Run all steps before every push.

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

**Pass**: exit 0, all tests pass, 0 failures.
**Fix**: read test output, fix code or test, re-run.

## 5. Smoke Test

```bash
./target/release/repo-inspect --repo . find-how "search" --depth 1
```

**Pass**: exit 0, file created in `.inspect/`.
**Fix**: check stderr for error chain.

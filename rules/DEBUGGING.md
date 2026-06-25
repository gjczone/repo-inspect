# Debugging Rules — repo-inspect

## Debugging Workflow

1. **Reproduce**: Run the failing command with the same args. Confirm the error.
2. **Isolate**: Narrow to the responsible module by checking the error chain.
3. **Fix**: Address root cause, not symptoms.
4. **Verify**: Re-run the exact failing command. Confirm exit 0.

## Logging

Run with `RUST_LOG` to enable verbose output:

```bash
RUST_LOG=debug repo-inspect --repo . find-how "search"
RUST_LOG=trace repo-inspect --repo owner/repo find-how "query"
```

Log levels: `error` (default) → `warn` → `info` → `debug` → `trace`.

## Error Chains

The binary uses `anyhow::Result` — errors propagate with `.context()`:

```
Error: Failed to fetch file tree for owner/repo
Caused by: HTTP 403 Forbidden: rate limit exceeded
```

Read the full chain top-to-bottom: the outermost error is the operation, inner errors are root causes.

## Common Issues

| Symptom | Likely Cause | Debug Command |
|---------|-------------|---------------|
| Binary panics | `unwrap()` on a fallible path | Check backtrace via `RUST_BACKTRACE=1` |
| Empty output | `.inspect/` write failed silently | Check stderr for error chain |
| Remote 403 | Rate limit / missing token | Set `GITHUB_TOKEN`, check API status |
| Remote 404 | Wrong owner/repo or private repo | Verify `gh repo view owner/repo` |
| Cache stale | 24h TTL expired | Pass `--refresh` to force fetch |
| Build error | Missing Rust toolchain | `rustup update stable && rustc --version` |
| Linker error | Missing system deps | `sudo apt install build-essential` |

## Binary Inspection

```bash
# Check binary health
./target/release/repo-inspect --version
./target/release/repo-inspect --help

# Verify bundled binary
ls -lh skills/repo-inspect/scripts/repo-inspect

# Strict rebuild (after cargo clean)
cargo clean && cargo build --release
```

## State Inspection

```bash
# Remote cache location
ls -la ~/.cache/repo-inspect/remote/

# Cache metadata
cat ~/.cache/repo-inspect/remote/<owner>-<repo>/meta.json

# Output files
ls -la .inspect/
cat .inspect/find-how-*.md
```

## Profiling (when debugging performance)

```bash
# Binary size
ls -lh target/release/repo-inspect

# Release build timing
cargo build --release --timings
```

## Discussion and Collaboration with the User

- Reproduce the bug before proposing a fix — **NEVER** guess.
- Share the exact reproduction command and error output.
- Propose fix with evidence (test output, error chain).

# Performance Rules — repo-inspect

## Performance Profile

- **Cold start**: Binary loads in < 100ms (stripped release build)
- **File traversal**: Limited by `ignore` crate walk speed (I/O bound)
- **Remote mode**: Latency dominated by GitHub API + network; files downloaded sequentially
- **Memory**: Proportional to cached file count + symbol graph size

## Binary Size

```bash
$ ls -lh target/release/repo-inspect
5.5M   # Release build, stripped + LTO
```

- Target: < 6 MB for release builds
- Optimizations: `opt-level = "z"`, `lto = true`, `strip = true`, `codegen-units = 1`

## Hotspots

| Area | Risk | Mitigation |
|------|------|------------|
| `search::FileFinder::walk()` | Large repos (>10k files) | `.gitignore` skipping, `MAX_FILES = 5000` cap |
| `remote::fetch_raw_file()` | Many files, sequential HTTP | Downloaded one-by-one; raw.githubusercontent.com (no rate limit) |
| `scan::parser` | Tree-sitter parsing per file | Lazy; only files matching query |
| `graph::pagerank` | Dense symbol graphs | Bounded iteration count |

## Remote Mode Performance

- First fetch: O(N) HTTP requests where N = source file count (capped at 5000)
- Subsequent: O(1) — cache hit, no network
- `--refresh`: O(N) again

## Optimization Rules

- **NEVER** optimize without profiling first. The binary is already fast enough for its use case.
- If remote mode is slow: parallelize file downloads (but keep it synchronous — no tokio).
- If symbol graph is large: add depth limits to traversal.
- If binary size grows: check for duplicate dependencies via `cargo bloat`.

## Profiling

```bash
# Build with timing info
cargo build --release --timings

# Check binary size contributors
cargo bloat --release 2>/dev/null || echo "install: cargo install cargo-bloat"

# Count dependencies
cargo tree | wc -l
```

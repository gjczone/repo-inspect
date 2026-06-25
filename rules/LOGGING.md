# Logging Rules — repo-inspect

## Logging Stack

| Crate | Role |
|-------|------|
| `log` | Logging facade — `info!`, `debug!`, `error!`, `warn!` macros |
| `env_logger` | Runtime — reads `RUST_LOG` env var |

## Usage

```rust
use log::{info, debug, warn, error};

info!("Using cached remote files for {}/{}", owner, repo);
debug!("Default branch for {}/{}: {}", owner, repo, branch);
warn!("Tree response truncated for {}/{}", owner, repo);
error!("Failed to fetch {}: {}", path, e);
```

## Log Levels

| Level | When |
|-------|------|
| `error` | Operation failed, recovery not possible |
| `warn` | Degraded but continuing (e.g., truncated API response) |
| `info` | Key operational events (cache hit/miss, file count) |
| `debug` | Detailed state (branch name, file paths, cache age) |
| `trace` | Not used |

## Rules

- **NEVER** use `eprintln!` or `println!` for debug/log output — use `log` crate macros.
- `eprintln!` is acceptable for user-facing progress messages (e.g., "Downloading 29 source files...").
- **NEVER** log tokens, secrets, or full file contents.
- URL logging: use `sanitize_url()` to truncate at 80 chars.
- Default log level: `info` (set in `main()` via `env_logger::Builder`).

## Enabling Debug Logs

```bash
RUST_LOG=debug repo-inspect --repo owner/repo find-how "query"
RUST_LOG=trace repo-inspect --repo . entries
```

## Production Considerations

- Release builds strip debug symbols (`strip = true` in `[profile.release]`)
- Log output is minimal at default `info` level
- No persistent log files — everything goes to stderr

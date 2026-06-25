# Data & State Rules — repo-inspect

## State Model

The binary is **stateless between invocations**. Each run is independent.

- **NEVER** introduce persistent state that survives between invocations.
- **NEVER** read files outside the `--repo` directory (except `.gitignore` rules and remote cache).
- `.inspect/` is the only output directory. **NEVER** write files outside `.inspect/`.

## Output Files

```
.inspect/
├── find-how-<query>.md       # Feature how-it-works results
├── trace-<symbol>.md          # Symbol trace results
├── entries.md                 # Entry point inventory
├── patterns.md                # Design pattern report
├── data.md                    # Data structure catalog
└── hotspots.md                # Hotspot analysis
```

## Remote Cache

```
~/.cache/repo-inspect/remote/
└── <owner>-<repo>/
    ├── meta.json              # { fetched_at, branch, file_count }
    └── <path>                 # Source files (repo-relative paths)
```

### Cache Lifecycle

- **TTL**: 24 hours (`CACHE_TTL = 86_400`)
- **Refresh**: `--refresh` flag forces re-fetch
- **Cleanup**: Cache dir is cleared before each fresh fetch (`fs::remove_dir_all`)
- **Eviction**: No automatic eviction — users manage manually

### meta.json Schema

```json
{
  "fetched_at": 1782397776,
  "branch": "main",
  "file_count": 29
}
```

## Input Data

| Source | Format | Location |
|--------|--------|----------|
| CLI args | `clap::Parser` | `src/cli.rs` |
| Local files | FS read via `ignore` crate | `--repo` directory |
| Remote files | GitHub API → raw.githubusercontent.com | `~/.cache/...` |
| Environment | `GITHUB_TOKEN` (optional) | Process env |

## Data Flow

```
CLI args → main() → RepoSpec resolution
                     ├── Local(PathBuf) → run_command(path)
                     └── Remote {owner, repo} → remote::prepare() → run_command(cache_path)
                                                      │
                                                      ├── Cache hit → return cache dir
                                                      └── Cache miss / --refresh → fetch → cache → return
```

## Security Boundaries

- `GITHUB_TOKEN` is read from env, **NEVER** logged or written to output
- URL sanitization: `sanitize_url()` truncates at 80 chars for log safety
- Remote files are treated as untrusted input — same as local files
- Cache directory is user-writable; no setuid/setgid concerns

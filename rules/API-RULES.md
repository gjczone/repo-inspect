# API Rules — repo-inspect

The CLI interface is the public API. There is no HTTP/REST/gRPC surface — this is a single binary invoked from the command line.

## CLI as API

- **NEVER** change a command name, flag name, or flag short form without backward compatibility.
- **NEVER** change the output format (`.inspect/` file structure, Markdown layout, JSON schema) without a major version bump.
- `--output json` mode MUST produce valid JSON.
- All subcommand flags are documented in `skills/repo-inspect/references/commands.md` — **NEVER** add a flag without updating it.

## Command Structure

```
repo-inspect [--repo <repo>] [--output md|json] [--out-dir <dir>] [--refresh] <command> [args]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--repo` | `RepoSpec` | `.` | Local path or `owner/repo` |
| `--output` | `md` / `json` | `md` | Output format |
| `--out-dir` | Path | `.inspect` | Output directory |
| `--refresh` | Flag | off | Force re-fetch remote cache |

## Subcommands

| Command | API Contract |
|---------|-------------|
| `find-how <query>...` | Query terms → scored file matches → `.inspect/find-how-<query>.md` |
| `trace <symbol>` | Symbol name → callers/callees trace → `.inspect/trace-<symbol>.md` |
| `entries` | All sources → entry point detection → `.inspect/entries.md` |
| `patterns` | All sources → pattern detection → `.inspect/patterns.md` |
| `data` | All sources → data structure extraction → `.inspect/data.md` |
| `hotspots` | git history → hotspot ranking → `.inspect/hotspots.md` |

## Output Filename Convention

Pattern: `<command>-<sanitized-query>.<ext>`

- Query is sanitized: non-alphanumeric chars replaced with `-`, max 64 chars
- Ext: `.md` (default) or `.json`
- **NEVER** change the sanitization logic without updating all consumers

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (bad args, missing repo, API failure) |
| 101 | Panic (should never happen in production) |

## Remote Mode API

When `--repo` is `owner/repo` format:
1. `remote::prepare(owner, repo, refresh)` → returns `PathBuf` to cache directory
2. All subsequent analysis runs on the cached directory as if it were local
3. Remote mode is transparent to command implementations — they only see the final `PathBuf`

## Backward Compatibility

- `--repo .` must always work identically
- Existing `.inspect/` output format must never change without version bump
- Adding a new subcommand is safe; removing or renaming one requires major version bump

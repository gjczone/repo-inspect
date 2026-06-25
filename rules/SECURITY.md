# Security Rules — repo-inspect

## Secrets Management

- **NEVER** hardcode tokens, keys, or credentials in source code.
- `GITHUB_TOKEN` is read from environment: `std::env::var("GITHUB_TOKEN").ok()`. It is stored as `Option<String>` and **NEVER** logged.
- URL sanitization: `sanitize_url()` truncates URLs to 80 chars — prevents token-bearing URLs from appearing in logs.

## Input Boundaries

| Boundary | Risk | Mitigation |
|----------|------|------------|
| CLI args (`--repo`) | User-supplied path | Validated by `RepoSpec::FromStr` and `Path::exists()` |
| `owner/repo` string | GitHub API injection | Parsed via `split('/')`, validated non-empty parts |
| Raw file content (GitHub) | Malicious source code | Treated as data only — parsed, not executed |
| Cached files | Stale or tampered | TTL enforced; `--refresh` for re-fetch |
| File paths (`--out-dir`) | Path traversal | Defaults to `.inspect/`; user-specified path via CLI |

## API Security

- GitHub API: authenticated via `Authorization: Bearer <token>` header when `GITHUB_TOKEN` is set
- Rate limits: 5000 req/h (authenticated), 60 req/h (unauthenticated)
- Error responses handled: 401 (bad token), 403 (rate limit), 404 (not found)
- Raw file fetch uses `raw.githubusercontent.com` (does not count against rate limits)

## File System Safety

- Output: **NEVER** writes outside `.inspect/` directory
- Cache: `~/.cache/repo-inspect/remote/` — standard XDG cache location
- File paths from remote: used as-is for local write; no traversal beyond cache root
- **NEVER** execute downloaded code — it's parsed and analyzed, never `eval`/`exec`/compiled

## Build Security

- Release builds: `opt-level = "z"`, `lto = true`, `strip = true`, `codegen-units = 1`
- Dependencies: all from crates.io, locked in `Cargo.lock`
- **NEVER** add a dependency without explicit justification

## Data Privacy

- No telemetry, no analytics, no network calls except GitHub API (remote mode)
- Local mode: zero network, reads only `--repo` directory
- `.inspect/` and cache are local files — user controls cleanup

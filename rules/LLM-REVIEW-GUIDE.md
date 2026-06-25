# LLM Review Guide — repo-inspect

Guidance for LLM agents performing code review on this project. Generated from repository evidence.

## DO NOT REPORT

The following are intentionally part of this project and MUST NOT be flagged as issues:

- **Synchronous HTTP**: `minreq` is used instead of `reqwest`/`hyper` — by design (no async runtime).
- **Sequential file downloads**: Remote mode downloads files one-by-one — acceptable for this tool's use case.
- **Unsafe code absence**: The project has zero `unsafe` blocks — this is correct.
- **No CI for private repos**: Private projects use `rules/LOCAL_CI.md` only — by design (GA minutes not free).
- **CLI-only, no web server**: This is a single binary, not a service — no HTTP listener expected.

## Review Focus Areas

| Area | What to Check |
|------|--------------|
| `src/remote/mod.rs` | Token handling (never logged), error propagation, cache TTL, URL sanitization |
| `src/cli.rs` | Backward compatibility (`--repo .`), `FromStr` correctness, flag docs |
| `src/search/mod.rs` | `.gitignore` respect, file count limits, path traversal |
| `src/commands/*` | Each command is self-contained, no cross-command imports |
| `src/main.rs` | Error handling in dispatch, no silent failures |

## Rust-Specific Checks

- [ ] No `unwrap()` or `expect()` on fallible operations (I/O, network, parse)
- [ ] All `match` arms handled or explicitly `_ => {}` with a comment
- [ ] `?` operator used with `.context()` for meaningful error messages
- [ ] No `eprintln!` — use `log` crate macros
- [ ] No `unsafe` blocks
- [ ] `cargo clippy -- -D warnings` passes (treat warnings as errors)

## Security Checks

- [ ] `GITHUB_TOKEN` never appears in log/error/output
- [ ] URL strings sanitized before logging (`sanitize_url()`)
- [ ] File paths validated before read/write
- [ ] No hardcoded credentials, tokens, or keys

## Test Coverage Checks

- [ ] New public function has at least one test
- [ ] Error paths are tested (not just happy path)
- [ ] No test depends on external network or specific repo state
- [ ] `cargo test` passes — 0 failures, 0 ignored

## Review Output Format

For each finding:
```
[SEVERITY] file:line — description
Fix: specific change
```

Severity: `CRITICAL` (security/data loss), `HIGH` (bug/crash), `MEDIUM` (tech debt), `LOW` (style/naming)

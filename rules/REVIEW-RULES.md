# REVIEW-RULES.md for repo-inspect

You are reviewing **repo-inspect**, a surgical codebase inspection CLI for AI agents — ask "how is X implemented?" and get compact, structured output in Markdown or JSON. This guide focuses your review on real bugs and reliability issues only.

## Project Context

- **What it is**: Rust CLI binary that inspects codebases locally (respecting `.gitignore`) or remotely via GitHub API, producing structured output to `.inspect/` for AI agent consumption.
- **Size**: 23 source files across 8 modules (`cli`, `main`, `commands/` 7 subcommand modules, `search`, `output`, `scan`, `remote`, `graph`, `git`), ~45 test functions. Growing project (10–50 files).
- **Runtime**: Rust 2024 edition (1.85+), single binary, no async runtime (sync HTTP via `minreq`).
- **Dependencies**: `clap` (CLI), `ignore` (`.gitignore`-aware traversal), `rayon` (parallel scan + remote downloads), `tree-sitter` + 4 grammars (rust, python, typescript, go), `minreq` (sync HTTPS), `serde`/`serde_json` (serialization), `anyhow`/`thiserror` (error handling), `log`/`env_logger` (logging).
- **Concurrency model**: Rayon `par_iter()` for CPU-bound tree-sitter parsing and parallel remote file downloads; `AtomicUsize` for lock-free counters in parallel contexts. No async, no `Mutex` held across parallel iterators.
- **State model**: Binary is stateless. Output written to `.inspect/`. Remote cache at `~/.cache/repo-inspect/remote/<owner>-<repo>/` with 24h TTL. Progressive scanning: Tier 1 (metadata only) → Tier 2 (search API + selective download) → Tier 3 (full download).
- **Key architecture**: 3-phase scan pipeline (serial I/O collect → per-language `CompiledQueries` compilation → rayon parallel parse). CLI interface IS the API — backward compatibility is critical. Dependency direction is strictly top-down: CLI → commands/remote → shared infra (search, output, scan, graph) → filesystem/network.

## Review Rules

### DO report these (P0 — must fix)

1. **Logic errors**: `unwrap()` on `Option` after a guard that appears safe but is fragile to refactoring — e.g., `unwrap()` after an `is_none()` check, or `unwrap()` after `starts_with()`. These were fixed in v0.1.2 (`let Some` / `if let Some` patterns) but can reappear. Float NaN: `partial_cmp().unwrap()` — fixed in v0.1.2 (`unwrap_or(Ordering::Equal)`). Always prefer `if let` / `let Some` over guard-then-unwrap.

2. **Type safety holes**: `SymbolKind::from_capture()` returns `None` for unrecognized capture names — caller must not silently drop the `None`. `detect_language()` returns `None` for unsupported file types — downstream code must handle this. Verify every call site handles `None` explicitly, not with `unwrap()`.

3. **Concurrency bugs (rayon)**: `prepare_lightweight()` in `src/remote/mod.rs` deletes entire cache directory (`fs::remove_dir_all`) before writing new data — if the process is killed mid-write, ALL cached data is lost with no recovery path. Parallel writes in `prepare()`: each file is written to a unique path but `fresh_files` HashMap is built across threads via collect-then-merge — verify no race on the `fresh_files` insert loop (currently serial after collect, so safe).

4. **Data corruption**: `prepare_lightweight()` clears-then-rewrites cache atomically at the directory level — still no partial-write protection at the directory level (individual file writes are now atomic via temp+rename, fixed in v0.1.2). `check_cache()` verifies freshness by TTL only, not file integrity (no checksums). Corrupted cached files will silently produce wrong analysis results until TTL expires.

5. **Security — command injection**: `src/commands/overview.rs:647` runs `git log` via `std::process::Command` with a repo path from user input. While `Command` passes args as separate strings (no shell expansion), verify the repo path cannot contain `--` flags that could alter `git log` behavior.

6. **Regression risk from shared infra changes**: `src/search/mod.rs` `is_source_file()` and `src/remote/mod.rs` `is_source_file()` are near-identical copies with duplicated extension lists (~30 extensions each). A change to one without updating the other causes silent divergence — local mode and remote mode would treat different file sets as source files. Any addition or removal of a file extension MUST update both copies.

### DO report these (P1 — reliability risk)

1. **Silent error swallowing**: `FileFinder::search()` previously caught walk errors with `Err(_) => return Vec::new()` — FIXED in v0.1.2 (now returns `Result`). Remote download partial failures: previously only `debug!()`-logged — FIXED in v0.1.2 (now has error rate threshold >10% → hard error, otherwise `warn!` + `eprintln!` user warning). Still watch for any new silent-swallow patterns.

2. **Missing error handling**: clock `duration_since(UNIX_EPOCH)` — FIXED in v0.1.2 (now uses `checked_sub` + `warn!` for clock anomalies). Watch for any new unchecked clock arithmetic or `unwrap_or_default` on system time.

3. **Inconsistent state from duplicated logic**: The two `is_source_file()` implementations in `src/search/mod.rs` and `src/remote/mod.rs` must stay in sync. Currently they are not structurally guaranteed to match — a reviewer must verify they contain the same extension list and skip-directory list. See P0 item 6.

4. **Edge cases — empty/null input**: `sanitize_filename()` in `src/output/mod.rs` produces an empty string for an all-symbol query (e.g., `"   "` or non-alphanumeric characters), leading to output filename `find-how-.md` or `find-how.md` — verify the `OutputWriter::new()` logic handles degenerate filenames. `scan_project()` silently skips files with zero detected language — ok for now but verify zero-file scan result doesn't panic downstream in overview/PageRank.

5. **Performance — hot path regression**: `src/search/mod.rs` reads every file into memory via `fs::read_to_string` and lowercases the entire content for scoring — O(total bytes) memory and CPU. `src/output/mod.rs` `write_markdown()` for symbols builds the entire markdown output as a `String` before a single `fs::write()` call — could OOM for very large symbol sets. `src/scan/mod.rs` collects all file contents into a single `Vec` before parallel parsing — memory peaks at O(total source bytes).

6. **Error message quality**: rate-limit detection via string matching — FIXED in v0.1.2 (now uses serde_json parsing of `RateLimitResponse`, error bodies truncated to 200 chars). Still watch for fragile string matching in new error-handling paths.

7. **CLI interface contract**: Any change to command names, flag names, output format, or filename sanitization logic breaks the API contract. `skills/repo-inspect/references/commands.md` MUST be updated in lockstep. `--output json` MUST produce valid JSON matching the documented struct shape.

### DO NOT report these (ignore — not useful)

- Code style, formatting, variable naming, line length, comment completeness.
- Rename suggestions, function-split suggestions — unless there is a concrete bug caused by the structure.
- Test coverage percentages, missing test categories.
- Dependency version suggestions (unless there is a known CVE).
- Linting-level suggestions (`let` vs `let mut`, `match` vs `if let`).
- Use of `eprintln!` for user-facing progress output — this is an intentional design choice for the CLI's stdout/stderr contract. `log` crate handles debug logging.
- Missing docs, missing comments — the project manages docs separately.
- Architecture opinions ("use trait instead of enum", "extract this into a module").
- Feature suggestions not currently implemented.

## Key Files to Review

### Tier 1 — Core Logic (highest risk)

| File | What to check |
|------|---------------|
| `src/main.rs` | 3-tier progressive scanning dispatch — verify every `Command` variant is matched and every branch handles `RepoSpec::Remote` correctly. `out_dir` resolution: relative paths are joined to repo dir, not cwd — verify this doesn't surprise callers. Missing `--full` flag fallthrough for commands not in Tier 1/2 dispatch. |
| `src/remote/mod.rs` | Cache safety: `prepare_lightweight()` `remove_dir_all` + rewrite is atomic only if process survives. `check_cache()` trusts TTL without file integrity check. Parallel `par_iter()` download with `AtomicUsize` counters — verify collect-then-write serial phase after `par_iter()` doesn't have data race on `fresh_files`. `is_source_file()` must stay in sync with `src/search/mod.rs` copy. Error handling in `fetch_raw_file()`: raw URL fallback to Contents API may double-count API rate limits when raw URL returns non-404 errors. |
| `src/search/mod.rs` | `FileFinder::search()`: now returns `Result` (fixed v0.1.2). Score uses `unwrap_or(Ordering::Equal)` (fixed v0.1.2). `is_source_file()` extension list must match `src/remote/mod.rs`. `extract_matching_lines()` hard-caps at 50 results — intentional but verify callers don't assume completeness. |
| `src/output/mod.rs` | `sanitize_filename()`: empty string output for all-symbol queries — verify `OutputWriter::new()` handles empty sanitized name. `write_markdown()` and `write_symbol_markdown()` both query filename stem for the title — this is fragile (`.find-how-trace.md` would produce wrong title). `out_dir` resolution: relative `out_dir` is joined to repo dir in `main.rs`, not here — verify callers pass the correct resolved path. |
| `src/scan/mod.rs` | `scan_project()`: all file contents collected into single `Vec` before parallel parse — memory O(total bytes). `find_symbols()` / `find_call_refs()` return references into `ScanResult` — caller must not drop `ScanResult` while references are live. `find_symbols()` sorts by exact match then prefix then contains, but uses only `cmp` on bool — equal-quality entries have nondeterministic ordering. |

### Tier 2 — State & Recovery

| File | What to check |
|------|--------------|
| `src/remote/mod.rs` (caching) | `check_cache()`: age check uses `saturating_sub` so clock skew toward past → cache never expires. Clock skew toward future → cache immediately expires. No checksum or manifest integrity check — a partially-written file is served as valid. `ensure_cached()` reads then writes meta with `unwrap_or()` default — if meta file is corrupted, it's silently replaced with a fresh one, losing timestamp data for other cached files. |
| `src/scan/parser.rs` (CompiledQueries) | `compile_queries()` returns `None` on query compilation failure — verify callers propagate this as a language-level skip, not a hard error. `parse_file_with_queries()` uses `tree_sitter::QueryCursor::new()` per file — correct (QueryCursor is not reusable across files). Symbol signature truncation at 200 chars via `chars().take(200)` — verify this doesn't break multi-byte UTF-8 boundaries (it doesn't, `chars()` is Unicode-safe). |

## How to Submit Findings

```
### [P0|P1] Short title

**File**: `src/path/to/file.rs:line`

**Problem**: Describe the bug or reliability risk in 2-3 sentences.

**Impact**: What actually goes wrong? When would it happen?

**Fix**: Suggest the minimal code change.
```

Skip any finding that does not meet the P0/P1 bar. Do not submit more than 15 findings total — prioritize the most impactful ones.

## Quick Sanity Checklist

- [ ] `cargo fmt --check` — zero diff
- [ ] `cargo clippy -- -D warnings` — exits 0, zero warnings
- [ ] `cargo build --release` — exits 0, binary at `target/release/repo-inspect`
- [ ] `cargo test` — all tests pass, zero failures
- [ ] `grep -rn "\.unwrap()" src/ --include="*.rs" | grep -v "cfg(test)" | grep -v "#\[test\]" | grep -v "_unwrap\b"` — review all `unwrap()` in production code paths (should only appear in `graph/builder.rs:416` and `graph/mod.rs:219` after v0.1.2 fixes)
- [ ] `grep -r "eprintln!" src/` — all `eprintln!` calls are user-facing progress output, not debug logging that should use `log` crate
- [ ] Verify `is_source_file()` in `src/search/mod.rs` and `src/remote/mod.rs` have identical extension lists and skip-directory lists
- [ ] `ls -lh target/release/repo-inspect` — binary < 6 MB
- [ ] `cargo run -- --repo . find-how "test" --depth 1` — exits 0, output in `.inspect/`
- [ ] `cargo run -- --repo . overview` — exits 0, output in `.inspect/`
- [ ] `grep -r "TODO\|FIXME\|HACK\|XXX" src/` — any leftover markers that should be addressed?
- [ ] Verify `cp target/release/repo-inspect skills/repo-inspect/scripts/` was run after build — bundled binary is current

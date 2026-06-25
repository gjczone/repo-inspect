# Coding Rules — repo-inspect

Rust 2024 edition. All rules come from evidence in the repository.

## Function Scope

- **NEVER** write a function that does more than one thing. If the name needs "and" to describe its purpose, split it.
- This rule applies only to new or modified functions within the task scope. **NEVER** proactively refactor existing functions on this basis.

## File Boundaries

- One file = one business concept. Any file with a generic name (`utils`, `helpers`, `common`, `misc`) that spans multiple unrelated domains is a boundary violation — regardless of line count.
- When a file directly touched by the task contains 2+ unrelated domains, extract each into its own file. **NEVER** proactively scan the codebase to clean this up.
- **NEVER** create a module file that only re-exports another module's symbols — inline the imports at call sites instead.

## Error Handling

- Every `match Err` / `?` error path **MUST** either handle the error with a log or propagate it. Empty error branches are forbidden.
- Log: what operation failed, the input context, and the original error message.
- Use `anyhow::Result` for application-level errors, `thiserror` for library-level structured errors.
- **NEVER** leave an `unwrap()` or `expect()` on a fallible operation that can fail under normal use — use `?` with `anyhow::Result`.

## Parallelism (rayon)

- Use `par_iter()` for CPU-bound parallel work (tree-sitter parsing, remote file downloads).
- Use `AtomicUsize` for lock-free counters in parallel contexts (e.g., progress tracking).
- **NEVER** hold a `Mutex` across a `par_iter()` — prefer atomics or collect-then-merge.
- Scan pipeline: serial I/O phase collects files, then rayon parallelizes the parse phase.

## Query Caching (CompiledQueries)

- `CompiledQueries` pre-compiles tree-sitter `Query` objects once, then reuses them across all files.
- **NEVER** call `tree_sitter::Query::new()` inside a per-file loop — compile once, pass as reference.
- Pattern: build `CompiledQueries` at scan start → pass `&CompiledQueries` into parallel parse workers.

## Dependencies

- **NEVER** add a new dependency without explicit justification in the PR body.
- **NEVER** introduce async runtime (tokio/async-std) — the project uses synchronous HTTP (`minreq`) and local file I/O.
- External library APIs → query `context7` MCP. **NEVER** guess API signatures.

## CLI API (the binary's public interface)

- The CLI interface IS the API. **NEVER** change a command name, flag, or output format without backward compatibility or a major version bump.
- Output filenames follow the pattern `<command>-<sanitized-query>.<ext>`. **NEVER** change the sanitization logic without updating consumers.
- `--output json` mode MUST produce valid JSON matching the `FindHowOutput` struct shape.

## Comments

- Comments must explain: business purpose, implementation logic, and edge cases. Use Chinese; avoid jargon.
- **NEVER** leave `eprintln!` debug output in committed code — use `log` crate macros (`info!`, `debug!`, `error!`).

## Naming

| Element | Convention | Example |
|---------|------------|---------|
| Module | snake_case | `find_how`, `mod.rs` |
| Function | snake_case | `run()`, `fetch_raw_file()` |
| Struct | PascalCase | `Args`, `RepoSpec` |
| Enum variant | PascalCase | `Remote { owner, repo }` |
| Constant | UPPER_SNAKE_CASE | `API_BASE`, `CACHE_TTL` |
| Crate | kebab-case | `repo-inspect` |

## Anti-Patterns

| Anti-Pattern | Detection | Fix |
|--------------|-----------|-----|
| `unwrap()` on fallible ops | `.unwrap()` on I/O, network, parse | Use `?` with anyhow |
| Empty error branch | `_ => {}` without log | Log or propagate |
| Subcommand stub | `unimplemented!()` / `todo!()` | Implement or leave out |
| `eprintln!` in production | Raw stderr prints | Use `log` crate |
| Magic paths | Hardcoded `/tmp/...` | Use `dirs_fallback()` or config |

## Change Discipline

- When replacing a component, function, or module: ① grep all references, ② update them, ③ delete the old file — all in the same change. No leftover references. No compatibility wrappers.
- **NEVER** modify the CLI interface (`src/cli.rs`) without updating `skills/repo-inspect/references/commands.md`.

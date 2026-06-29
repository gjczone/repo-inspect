# Coding Rules — repo-inspect

Rust 2024 edition. Project-specific conventions — things an LLM would not know from general Rust knowledge. Every rule cites evidence from the repository.

---

## 1. Layer Boundaries

Import direction is top-down only. Evidence: `rules/ARCHITECTURE.md` lines 26-29; verified by inspecting `use` statements across the codebase.

| Layer | May import from |
|-------|----------------|
| `cli` | Nothing (standalone clap definitions) |
| `main` | `cli`, `remote`, any `commands/` |
| `commands/*` | `search`, `output`, `scan`, `graph` |
| `remote` | `search`, `scan` |
| `search`, `output`, `scan`, `graph` | Each other; crates only |

- **NEVER** import `cli` or `main` from `commands/` or lower.
- **NEVER** cross-import between command modules (e.g., `find_how` importing from `trace`).

## 2. New Subcommand Registration Pattern

Every new subcommand follows a fixed 4-step registration path. Evidence: `src/cli.rs` (Command enum), `src/main.rs` (match dispatch), `rules/ARCHITECTURE.md` lines 56-61.

1. Create `src/commands/<name>.rs` — implement logic with a public `run()` function
2. Register in `src/cli.rs`: add variant to `Command` enum, add field to `Args` struct
3. Register in `src/main.rs`: add match arm dispatching to the new `run()`
4. Add a smoke test in the new module file

- **NEVER** implement a subcommand without wiring it through all three files.
- **NEVER** leave `unimplemented!()` or `todo!()` in a registered command variant.

## 3. Parallelism Rules

CPU-bound work uses rayon. Evidence: `src/scan/mod.rs` (3-phase pipeline with `par_iter`), `src/remote/mod.rs` (parallel downloads with `par_iter`).

- Use `par_iter()` for tree-sitter parsing and remote file downloads.
- Use `AtomicUsize` for lock-free counters in parallel contexts (e.g., progress tracking).
- **NEVER** hold a `Mutex` across a `par_iter()` — prefer atomics or collect-then-merge.
- Scan pipeline: serial I/O collects files → rayon parallelizes the parse phase.
- Remote pipeline: serial GitHub API tree fetch → rayon `par_iter` parallel raw downloads.

## 4. CompiledQueries Caching

Tree-sitter `Query` objects are compiled once per language and reused across all files. Evidence: `src/scan/mod.rs` (3-phase pipeline description), `src/scan/parser.rs`.

- Build `CompiledQueries` once at scan start per language.
- Pass `&CompiledQueries` into parallel parse workers — **NEVER** recompile per file.
- **NEVER** call `tree_sitter::Query::new()` inside a per-file loop.

## 5. Dependencies

Evidence: `Cargo.toml` (no async runtime dependency; `minreq` for sync HTTP), AGENTS.md.

- **NEVER** add a new dependency without explicit justification in the PR body.
- **NEVER** introduce an async runtime (`tokio`, `async-std`). The project uses synchronous HTTP (`minreq`) and blocking file I/O.
- External library APIs → query `context7` MCP. **NEVER** guess API signatures.

## 6. CLI API Stability

The CLI interface IS the public API consumed by external agents via the skill. Evidence: `src/cli.rs` (Args/Command definitions), AGENTS.md.

- **NEVER** rename a command variant or CLI flag without backward compatibility.
- **NEVER** change output format shape (`FindHowOutput` struct) without updating consumers.
- Output filenames follow `<command>-<sanitized-query>.<ext>`. **NEVER** change sanitization logic (`src/output/mod.rs`) without updating consumers.
- When modifying `src/cli.rs`, update `skills/repo-inspect/references/commands.md` in the same change.

## 7. Error Handling

Evidence: `Cargo.toml` (`anyhow` + `thiserror`), `src/main.rs` (`anyhow::Result`), grep for `log::` macros across source files.

- Application-level: use `anyhow::Result<T>` with `?` propagation.
- Library-level (shared error types): use `thiserror` derive macros.
- Logging: use `log` crate macros (`info!`, `debug!`, `error!`, `warn!`).
- User-facing progress output: use `eprintln!` (stderr) — this is a deliberate design choice to avoid mixing progress with stdout machine output.
- **NEVER** use `println!` in production code (reserved for structured output to stdout).
- Every `match Err` / `?` error path **MUST** log or propagate. Empty error branches (`_ => {}`) are forbidden.
- **NEVER** leave `unwrap()` or `expect()` on operations that can fail under normal use (I/O, network, parsing).

## 8. Comments

Evidence: project convention stated in AGENTS.md.

- Comments explain business purpose, implementation logic, and edge cases.
- Write comments in Chinese; keep code identifiers, commands, and technical terms in English.

## 9. Change Discipline

Evidence: project convention, verified by module structure and import graphs.

- When replacing a component, function, or module: ① grep all callers, ② update every reference, ③ delete the old file — all in one change. No compatibility wrappers. No leftover `pub use` re-exports.

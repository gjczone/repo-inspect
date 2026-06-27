# ARCHITECTURE.md

## Layer Boundaries

| Layer | May import from | Must NOT import from |
|-------|----------------|---------------------|
| `cli` | clap, std | any `crate::*` |
| `main` | `cli`, all `mod` declarations | — |
| `commands/` | `cli`, `output`, `scan`, `search`, `graph` | other commands, `main`, `remote` |
| `remote/` | std, serde, rayon, anyhow, log | any `crate::*` module |
| `output/` | `cli` (OutputFormat), `scan`, `search` | `commands/`, `main`, `graph` |
| `scan/` | rayon, ignore, std | `cli`, `commands/`, `output`, `search`, `graph` |
| `search/` | ignore, std | `cli`, `commands/`, `scan`, `output`, `graph` |
| `graph/` | `scan` (parser types + ScanResult) | `cli`, `commands/`, `output`, `search` |

**Evidence**: `grep "^use crate::" src/**/*.rs` — every import matches this table. No cross-command imports (`commands/find_how` never imports `commands/trace`). No upward imports (`scan` never imports `output` or `cli`).

## Dependency Direction

**Top-down only**: `cli → main → commands → {scan, search, graph} → output → filesystem`

- `cli` is the pure definition layer — no internal dependencies.
- `main` is the router — resolves repo, calls `remote::prepare` for remote repos, dispatches to commands.
- `commands/` are the business logic layer — one module per subcommand, each imports shared infra.
- `{scan, search, graph}` are the compute layer — CPU-bound work with no knowledge of CLI or output.
- `remote/` is an isolated network layer — called only by `main`, never by commands.
- `output/` is the presentation boundary — the only module that writes to `.inspect/`.

## Key Modules

| Module | Role | Why it matters |
|--------|------|----------------|
| `cli` | CLI arg definitions + `RepoSpec` enum | Every subcommand starts here; args define the public API |
| `main` | Env init + repo resolution + dispatch | The sole entry point; `remote::prepare` branching happens here |
| `remote` | GitHub API: tree fetch + parallel raw download + cache | Isolated from all other modules; 1091 lines, most complex module |
| `scan` | 3-phase tree-sitter pipeline: serial I/O → Query compile → rayon parallel parse | Core analysis engine; `CompiledQueries` pattern avoids per-file recompilation |
| `search` | `.gitignore`-aware file walk + content grep via `ignore` crate | Used by `find-how` for L1 text fallback when L2 tree-sitter misses |
| `output` | Markdown / JSON formatting + `.inspect/` file write | Single presentation layer; OutputFormat enum drives all formatting |
| `graph` | Symbol graph: builder + PageRank + traversal | Used by `trace` and `hotspots`; imports `scan::ScanResult` only |
| `commands/` | 7 subcommand modules (find_how, trace, entries, patterns, data, hotspots, overview) | Each is self-contained; shared logic lives in scan/search/graph, not copied |

## Architectural Decisions

- **rayon for parallel scan** — tree-sitter parsing is the CPU bottleneck; `par_iter()` on all collected files avoids mutex contention with `AtomicUsize` counters. Not using async because there's no I/O in the parse phase.
- **minreq sync HTTP** — remote mode only hits 2 GitHub API endpoints (tree + raw file fetch). No streaming, no long-lived connections. Sync is simpler and avoids pulling in an async runtime for ~10 network calls per invocation.
- **3-tier progressive remote scanning** — `prepare` first tries full tree (fast, 1 API call), falls back to lightweight (filtered tree walk) on large repos, then to selective (targeted file fetch) on rate limits. Avoids hitting GitHub API limits on huge repos.
- **`CompiledQueries` per-language caching** — tree-sitter Query compilation is O(grammar size); compile once per language detected, reuse across all files of that language. Measurable 3-5x speedup vs per-file compilation.
- **No cross-command imports** — each command module is an island. If two commands share logic, it lives in `scan`, `search`, or `graph`. This prevents coupling between unrelated features.
- **Remote isolated from commands** — `main` handles all `remote::prepare` calls before dispatch; commands never know or care whether files came from local disk or GitHub. Simplifies testing (no network mocking in command tests).

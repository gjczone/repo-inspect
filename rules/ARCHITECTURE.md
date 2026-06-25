# Architecture Rules — repo-inspect

## Layer Model

```
┌─────────────────────────────────────┐
│              CLI (clap)             │  ← cli.rs — Args, RepoSpec, Command
├─────────────────────────────────────┤
│            main() router            │  ← main.rs — resolve repo → dispatch
├─────────────────────────────────────┤
│    ┌──────────┐    ┌──────────┐     │
│    │ commands/│    │  remote/ │     │  ← Business logic
│    │ 7 mods   │    │  mod.rs  │     │
│    └────┬─────┘    └────┬─────┘     │
├─────────┼───────────────┼───────────┤
│    ┌────┴───────────────┴─────┐     │
│    │      Shared infra       │     │  ← search, output, scan, graph
│    └──────────────────────────┘     │
├─────────────────────────────────────┤
│         Filesystem / Network        │  ← I/O boundary
└─────────────────────────────────────┘
```

## Dependency Direction

- **Top-down only**: CLI → commands/remote → search/output/scan → filesystem
- **NEVER** import `cli` or `main` from `commands/` or lower
- **NEVER** cross-import between commands (e.g., `find_how` importing from `trace`)
- Shared infrastructure (`search`, `output`, `scan`, `graph`) may be imported by any command

## Module Responsibilities

| Module | One-line responsibility |
|--------|------------------------|
| `cli` | CLI arg definitions + `RepoSpec` enum + `FromStr` |
| `main` | Env logger init + repo resolution + command dispatch |
| `commands/find_how` | Keyword search → file scoring → extract + format |
| `commands/trace` | Symbol lookup → caller/callee traversal → output |
| `commands/entries` | Entry point detection across all source files |
| `commands/patterns` | Design pattern heuristic detection |
| `commands/data` | Data structure / type definition extraction |
| `commands/hotspots` | Git history → change frequency ranking |
| `remote` | GitHub API: tree fetch, raw download, cache management |
| `search` | `.gitignore`-aware file walk + content grep |
| `output` | Markdown / JSON formatting + `.inspect/` write |
| `scan` | Tree-sitter parsing + symbol extraction |
| `graph` | Symbol graph: builder + PageRank + traversal |

## File Count Rule

- One file = one business concept
- Current: 21 source files across 8 modules
- If a module exceeds 500 lines, consider splitting

## Adding a Module

1. Create `src/<name>/mod.rs`
2. Implement the logic
3. Add `mod <name>;` to `src/main.rs`
4. Wire into CLI (if new subcommand): `src/cli.rs` + `src/main.rs` match branch
5. Add smoke test
6. Update this file's module table

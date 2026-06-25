# repo-inspect

Surgical codebase inspection CLI — built for AI agents to quickly understand how specific features and patterns are implemented in open-source projects.

Instead of dumping the entire repo into an LLM's context window, `repo-inspect` surgically extracts only what you need.

## Commands

| Command | What it does |
|---------|-------------|
| `find-how <query>` | Find how a specific feature/technique is implemented |
| `trace <symbol>` | Trace callers and callees of a function/type |
| `entries` | Find all entry points (CLI, HTTP, events, plugins) |
| `patterns` | Detect design patterns and conventions |
| `data` | Extract core data structures and schemas |
| `hotspots` | Identify most-changed and most-complex files |

## Output

Results are written to `.inspect/` (gitignored). Each query produces a single file:

```
.inspect/
├── find-how-middleware.md     # How middleware is implemented
├── trace-applyMiddleware.md   # Call chain for applyMiddleware
└── entries.md                 # All entry points found
```

## Usage (from an AI agent)

```bash
# Clone a repo and inspect it
gh repo clone reduxjs/redux -- --depth 1
cd redux
repo-inspect --repo . find-how "middleware plugin" --depth 2

# Output: .inspect/find-how-middleware_plugin.md
# Agent reads: Read .inspect/find-how-middleware_plugin.md
```

## Install

```bash
cargo install --path .
```

Or use the pre-built binary from releases.

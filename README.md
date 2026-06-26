# repo-inspect

Surgical codebase inspection for AI agents. Ask *"how is X implemented?"* and get a compact, structured answer — no API keys, no network calls, zero cost.

## What it does

AI agents waste context windows dumping entire repositories. `repo-inspect` fixes this by surgically extracting only what you need:

```
Agent: "How does Redux implement middleware?"
  ↓
repo-inspect find-how "middleware enhancer compose"
  ↓
.inspect/find-how-middleware.md   ← 40 lines, specific files + line numbers
  ↓
Agent reads the file, answers with precision
```

The Rust binary runs locally, respects `.gitignore`, and writes compact Markdown to `.inspect/`.

## Install

```bash
npx skills add https://github.com/gjczone/repo-inspect --skill repo-inspect
```

This installs the skill + bundled binary. The binary works on Linux x86_64. For other platforms, build from source.

## Commands

| Command | What it does |
|---------|-------------|
| `overview` | Single-command project spine: languages, deps, PageRank, structure |
| `find-how <query>` | Find how a specific feature/technique is implemented |
| `trace <symbol>` | Trace callers and callees of a function/type |
| `entries` | Find all entry points (CLI, HTTP, events, plugins) |
| `patterns` | Detect design patterns and conventions |
| `data` | Extract core data structures and schemas |
| `hotspots` | Identify most-changed and most-complex files |

## Output

Results land in `.inspect/` (add to `.gitignore`). Each query produces one compact file:

```
.inspect/
├── overview.md
├── overview-graph.md
├── find-how-middleware.md
├── trace-applyMiddleware.md
└── entries.md
```

## Quick Start

```bash
# 1. Install the skill
npx skills add https://github.com/gjczone/repo-inspect --skill repo-inspect

# 2. Clone any repo and ask a question
gh repo clone reduxjs/redux -- --depth 1
cd redux

# 3. Agent invokes the skill → subagent runs:
repo-inspect --repo . find-how "middleware" --depth 2

# 4. Results in .inspect/find-how-middleware.md
```

## Build from Source

```bash
git clone https://github.com/gjczone/repo-inspect
cd repo-inspect
cargo build --release
cp target/release/repo-inspect skills/repo-inspect/scripts/
```

Requires Rust 1.85+.

## Why not repomix / zread / code2prompt?

| Tool | Approach | repo-inspect difference |
|------|----------|------------------------|
| repomix | Dump entire repo into one file | Surgical: ask one question, get one answer |
| zread | LLM-generated wiki (needs API key) | Zero API keys, local-only |
| code2prompt | Full codebase → single prompt | Structured `.inspect/` files, layered consumption |

`repo-inspect` is for when you know *what* you want to learn — not for exhaustive documentation.

## Project Structure

```
repo-inspect/
├── skills/repo-inspect/       # The skill
│   ├── SKILL.md
│   ├── scripts/repo-inspect   # Pre-built binary
│   └── references/commands.md
├── src/                       # Rust source
├── Cargo.toml
└── AGENTS.md                  # Agent instructions
```

## License

MIT

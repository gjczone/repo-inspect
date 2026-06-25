# repo-inspect

Surgical codebase inspection for AI agents — ask "how is X implemented?" and get a compact, structured answer. No API keys, no network calls, zero cost.

Built for the `repo-inspect` skill. The Rust binary lives in `skills/repo-inspect/scripts/repo-inspect`.

## Install the Skill

```bash
npx skills add https://github.com/gjczone/repo-inspect --skill repo-inspect
```

This installs the `repo-inspect` skill into your agent's skills directory. The binary (`repo-inspect`) is bundled with the skill — no separate install needed.

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

Results are written to `.inspect/` (gitignored). Each query produces a single compact file:

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

## Build from Source

```bash
git clone https://github.com/gjczone/repo-inspect
cd repo-inspect
cargo build --release
cp target/release/repo-inspect skills/repo-inspect/scripts/
```

Requires Rust 1.85+.

## Project Structure

```
repo-inspect/
├── skills/repo-inspect/       # The skill (installed by `npx skills add`)
│   ├── SKILL.md               # Skill definition & workflow
│   ├── scripts/
│   │   └── repo-inspect       # Pre-built binary (2.0 MB)
│   └── references/
│       └── commands.md        # Full command reference
├── src/                       # Rust source
│   ├── main.rs
│   ├── cli.rs
│   ├── commands/              # Subcommand implementations
│   ├── search/                # File search engine (ignore crate)
│   ├── output/                # Markdown + JSON formatting
│   └── git/                   # Git integration (gix crate)
├── Cargo.toml
└── README.md
```

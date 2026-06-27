# repo-inspect

An AI agent skill for reverse-engineering and researching open-source projects. Ask "how is X implemented?" and get compact, structured output — no need to dump the entire codebase into context.

## Installation

```bash
npx skills add https://github.com/gjczone/repo-inspect --skill repo-inspect
```

The binary is also available via Cargo:

```bash
cargo install repo-inspect
```

## What It Does

repo-inspect is a **skill** that gives AI agents surgical access to any codebase. Instead of reading thousands of irrelevant lines, the agent asks a focused question and gets exactly the relevant code — ranked, with line numbers and context.

| Command | When to use | What it does |
|---------|-------------|--------------|
| `overview` | First time opening a repo | One-page project spine: languages, structure, key modules, entry points |
| `find-how` | "How is X implemented?" | Searches codebase, returns ranked files with matched lines ± context |
| `trace` | "What calls this function?" | Follows call chains — callers and callees of a symbol |
| `entries` | "Where does execution start?" | Detects entry points: CLI commands, HTTP routes, event handlers |
| `patterns` | "What design patterns are used?" | Heuristic detection of common patterns |
| `data` | "What data structures exist?" | Extracts type definitions, structs, interfaces, enums |
| `hotspots` | "What files change most often?" | Ranks files by git change frequency |

All output lands in `.inspect/` — compact Markdown (default) or JSON.

## Quick Examples

```bash
# Reverse-engineer how auth works in any repo
repo-inspect --repo ./some-project find-how "authentication"

# Research a remote repo without cloning
repo-inspect --repo gjczone/repo-inspect overview

# Trace call chain of a function
repo-inspect --repo . trace "handle_request" --depth 3
```

## Remote Mode

Inspect any public GitHub repository without cloning — just use `owner/repo`:

```bash
repo-inspect --repo rust-lang/rust find-how "async"
```

Fetches file trees and raw files via GitHub API. Set `GITHUB_TOKEN` to avoid rate limits. Results cached locally for 24 hours.

## Supported Languages

| Language | Tree-sitter parsing |
|----------|-------------------|
| Rust | ✓ |
| Python | ✓ |
| TypeScript | ✓ |
| Go | ✓ |

Other languages work with text-level search even without tree-sitter grammar support.

## Credits

Built with [tree-sitter](https://tree-sitter.github.io/) for structured code parsing.

## License

[MIT](LICENSE)

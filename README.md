# repo-inspect

Surgical codebase inspection CLI for AI agents — ask "how is X implemented?" and get compact, structured output.

## Installation

```bash
npx skills add https://github.com/gjczone/repo-inspect --skill repo-inspect
```

The binary is also available via Cargo:

```bash
cargo install repo-inspect
```

## Core Commands

| Command | When to use | What it does |
|---------|-------------|--------------|
| `overview` | First time opening a repo | Prints a one-page project spine: structure, key modules, entry points |
| `find-how` | "How is X implemented?" | Searches codebase for a keyword, returns ranked files with matched lines |
| `trace` | "What calls this function?" | Follows call chains from a symbol through callers and callees |
| `entries` | "Where does execution start?" | Detects entry points — main functions, route handlers, command handlers |
| `patterns` | "What design patterns are used?" | Heuristic detection of common patterns (singleton, factory, observer) |
| `data` | "What data structures exist?" | Extracts type definitions, structs, interfaces, and enums |
| `hotspots` | "What files change most often?" | Ranks files by change frequency from git history |

All output lands in `.inspect/` as Markdown (default) or JSON (`--output json`).

## Quick Examples

```bash
# Inspect a local repo — how is authentication implemented?
repo-inspect --repo ./my-project find-how "authentication"

# Inspect a remote GitHub repo — get the project overview
repo-inspect --repo gjczone/repo-inspect overview

# Trace callers of a specific function
repo-inspect --repo . trace "handle_request" --depth 3

# Find change hotspots, refresh git data
repo-inspect --repo . hotspots --full
```

## Remote Mode

Prefix the repo with `owner/repo` to inspect any public GitHub repository without cloning:

```bash
repo-inspect --repo rust-lang/rust find-how "async"
```

Remote mode fetches file trees and raw files via the GitHub API. Set `GITHUB_TOKEN` to avoid rate limits. Results are cached locally for 24 hours.

## Supported Languages

| Language | Overview | find-how | trace | patterns | data |
|----------|----------|----------|-------|----------|------|
| Rust | ✓ | ✓ | ✓ | ✓ | ✓ |
| Python | ✓ | ✓ | ✓ | ✓ | ✓ |
| TypeScript | ✓ | ✓ | ✓ | ✓ | ✓ |
| Go | ✓ | ✓ | ✓ | ✓ | ✓ |

Other languages work with text-level search (`find-how` text fallback) even without tree-sitter grammar support.

## Credits

Built with [tree-sitter](https://tree-sitter.github.io/) for structured code parsing.

## License

[MIT](LICENSE)

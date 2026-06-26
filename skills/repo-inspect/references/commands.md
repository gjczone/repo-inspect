# repo-inspect Command Reference

Complete reference for each subcommand of the `repo-inspect` binary.

---

## find-how

Search how a specific feature, technique, or concept is implemented.

```bash
# 本地仓库
repo-inspect --repo <path> find-how "<query>" [--depth 1-3]

# 远程仓库（无需克隆）
repo-inspect --repo owner/repo find-how "<query>" [--depth 1-3] [--refresh]
```

**How it works**:
1. Splits query into keywords
2. Walks the repo respecting `.gitignore`
3. Scores files by: filename match × 3 + content match count
4. Extracts matching lines with ±2 lines context
5. Outputs grouped by directory, sorted by relevance

**Depth levels**:
- `1`: Core files only (highest-scoring matches)
- `2`: Core + direct callers/references (default)
- `3`: Full chain — all related files

**Example**:
```bash
repo-inspect --repo ./redux find-how "middleware enhancer compose" --depth 2
# → .inspect/find-how-middleware_enhancer_compose.md
```

**Output structure** (Markdown):
```markdown
# Inspection: find-how-middleware

**Files found**: 5

### src/
_Key files: applyMiddleware.ts, compose.ts_

#### `applyMiddleware.ts`
L12: `export default function applyMiddleware(...)`
L15: `const chain = middlewares.map(middleware => middleware(middlewareAPI))`
L18: `return compose(...chain)(store.dispatch)`
```

---

## trace

Trace callers and callees of a symbol.

```bash
repo-inspect --repo <repo> trace <symbol> [--direction callers|callees|both]
```

**Example**:
```bash
repo-inspect --repo ./redux trace createStore --direction both
# → .inspect/trace-createStore.md
```

---

## entries

Find all entry points to the codebase.

```bash
repo-inspect --repo <repo> entries [--kind cli|http|event|plugin|all]
```

**Examples**:
```bash
repo-inspect --repo ./cli-tool entries --kind cli
repo-inspect --repo ./webapp entries --kind http
```

---

## patterns

Detect design patterns, conventions, and idioms.

```bash
repo-inspect --repo <repo> patterns [--category creational|structural|behavioral|concurrency]
```

---

## data

Extract core data structures, type definitions, and schemas.

```bash
repo-inspect --repo <repo> data [--name <type-or-module>]
```

---

## hotspots

Identify the most-changed and most-complex files.

```bash
repo-inspect --repo <repo> hotspots [--count 10]
```

Uses git history (`gix` crate) to rank files by:
- Commit frequency (churn)
- Recent changes
- File size / complexity

---

## overview

Single-command project spine: get a holistic architecture overview in one call.

```bash
# 本地仓库
repo-inspect --repo <path> overview [--filter <keyword>]

# 远程仓库（轻量级模式，只拉元数据不下载源文件）
repo-inspect --repo owner/repo overview [--filter <keyword>] [--refresh]
```

**How it works**:
1. Scans the project for symbols (tree-sitter) or uses cached metadata (remote lightweight)
2. Builds the symbol dependency graph and calculates PageRank
3. Extracts dependencies from config files (Cargo.toml, package.json, go.mod, pyproject.toml)
4. Collects recent git changes or commits from GitHub API
5. Produces a structured overview with 8 sections

**Output sections**:
- **Summary stats**: N symbols across M files (L languages)
- **Language Support**: file count per language
- **Key Dependencies**: extracted from project config files
- **Recent Changes**: last 10 commits
- **Top Files by PageRank**: file-level importance ranking
- **Entry Points**: high-PageRank functions, types, and CLI mains
- **Module Structure**: 2-level directory tree with file counts
- **Complexity Hotspots**: symbol density × PageRank score
- **Suggested Reading Order**: top 5 files to start reading

**Modes**:
- **Full mode** (local repo): full tree-sitter scan + graph + PageRank
- **Lightweight mode** (remote repo): metadata only via GitHub API (tree + README + configs + commits), zero source file downloads

**Example**:
```bash
repo-inspect --repo . overview
# → .inspect/overview.md

repo-inspect --repo . overview --filter graph
# → .inspect/overview-graph.md

repo-inspect --repo owner/repo overview
# → .inspect/overview.md (lightweight, ~2s)
```

---

## Common Options

| Option | Default | Description |
|--------|---------|-------------|
| `--repo <repo>` | `.` | Repository: local path (e.g., `.`) or remote GitHub (e.g., `owner/repo`) |
| `--output md` | `md` | Output format: `md` (Markdown) or `json` |
| `--out-dir <path>` | `.inspect` | Where to write output files |
| `--refresh` | Off | Force re-fetch remote repo, bypass 24h cache (remote mode only) |
| `--full` | Off | Force full download of all source files in remote mode, skip progressive scanning |

### Remote Mode (Three-Tier Progressive Scanning)

When `--repo` is given in `owner/repo` format, the binary picks the optimal fetch strategy:

| Tier | Commands | Strategy | Time |
|------|----------|----------|------|
| **Tier 1** (Lightweight) | `overview` | GitHub API: tree + README + configs + commits. Zero source files. | ~2s |
| **Tier 2** (Selective) | `find-how`, `trace` | GitHub Search API → locate matching files → download only those (5-30 files). Falls back to Tier 3 if Search API unavailable. | ~5-10s |
| **Tier 3** (Full) | `entries`, `patterns`, `data`, `hotspots` + `--full` flag | Download all source files. Incremental cache: only re-downloads changed files. | ~30s first, ~2s cached |

**Cache**: `~/.cache/repo-inspect/remote/{owner}-{repo}/` with 24h per-file TTL. Each file tracked individually — subsequent runs only download new or changed files.

Requires `GITHUB_TOKEN` env var for higher rate limits (optional, but recommended).

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success — results written to `.inspect/` |
| 1 | Error — check stderr for details |

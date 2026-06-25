# repo-inspect Command Reference

Complete reference for each subcommand of the `repo-inspect` binary.

---

## find-how

Search how a specific feature, technique, or concept is implemented.

```bash
repo-inspect --repo <path> find-how "<query>" [--depth 1-3]
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
repo-inspect --repo <path> trace <symbol> [--direction callers|callees|both]
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
repo-inspect --repo <path> entries [--kind cli|http|event|plugin|all]
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
repo-inspect --repo <path> patterns [--category creational|structural|behavioral|concurrency]
```

---

## data

Extract core data structures, type definitions, and schemas.

```bash
repo-inspect --repo <path> data [--name <type-or-module>]
```

---

## hotspots

Identify the most-changed and most-complex files.

```bash
repo-inspect --repo <path> hotspots [--count 10]
```

Uses git history (`gix` crate) to rank files by:
- Commit frequency (churn)
- Recent changes
- File size / complexity

---

## Common Options

| Option | Default | Description |
|--------|---------|-------------|
| `--repo <path>` | `.` | Path to repository root |
| `--output md` | `md` | Output format: `md` (Markdown) or `json` |
| `--out-dir <path>` | `.inspect` | Where to write output files |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success — results written to `.inspect/` |
| 1 | Error — check stderr for details |

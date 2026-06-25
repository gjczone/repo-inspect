---
name: repo-inspect
description: "User-invoked surgical codebase inspection. Invoke when the user says 'how does this project implement X', 'find how Y works in this codebase', 'what patterns does this repo use', 'inspect this codebase for Z', 'trace the call chain of', or 'analyze the architecture of this project'. Also invoke proactively when entering an unfamiliar repository — if .inspect/ exists, read it directly; if not, offer to run repo-inspect. This skill uses a Rust binary (scripts/repo-inspect) to surgically extract relevant code instead of dumping the entire repo. The binary runs locally, respecting .gitignore, with zero API calls. Output is written to .inspect/ (gitignored). Do NOT invoke for general code review (use code-review skill), documentation generation (use project-init), or TDD (use test-driven-development)."
---

# Repo Inspect

Surgically inspect a codebase to understand how specific features, patterns, or techniques are implemented. Uses a local Rust binary — no API keys, no network calls, zero cost.

## Core Principle

**Don't dump the entire repo into context. Ask a surgical question, get a surgical answer.** The binary `repo-inspect` searches, traces, and extracts only the relevant code. The main agent reads the compact `.inspect/` output — not 10,000 lines of irrelevant source.

## Prerequisites

- `scripts/repo-inspect` — the Rust binary. Must be executable. If missing, build from source: `cargo build --release && cp target/release/repo-inspect skills/repo-inspect/scripts/`
- For local repos: the target repository must be cloned on disk
- For remote repos: set `GITHUB_TOKEN` environment variable (optional, but strongly recommended to avoid rate limiting). The binary fetches files from GitHub API without cloning — zero local disk space for the target repo.

## Workflow

### Step 0: Check for existing inspection results

Check if `.inspect/` exists in the target repo. If it does, read the relevant file directly — do NOT re-run the binary.

```bash
ls .inspect/
```

If results for the current question already exist, read them and answer. Skip to Step 4.

### Step 1: Decide local vs remote mode

The `--repo` flag auto-detects the mode:
- **Local**: pass a filesystem path, e.g., `--repo .` or `--repo /path/to/repo`
- **Remote**: pass `owner/repo` format, e.g., `--repo gjczone/repo-inspect` — the binary fetches source files from GitHub API and caches them under `~/.cache/repo-inspect/remote/` with a 24h TTL. Use `--refresh` to force re-fetch.

If you already have the repo cloned locally, use local mode for speed. Use remote mode when you don't want to clone.

### Step 2: Choose the right command

| User asks | Command |
|-----------|---------|
| "How does X work?" / "How is Y implemented?" | `find-how "<query>"` |
| "Who calls this function?" / "Trace the call chain" | `trace <symbol>` |
| "What are the entry points?" / "How do I use this?" | `entries` |
| "What patterns does this use?" / "Design patterns" | `patterns` |
| "What data structures?" / "Schema" / "Types" | `data` |
| "What are the hotspots?" / "Most changed files" | `hotspots` |

### Step 3: Run repo-inspect

**Step 3a: Locate the binary.** The binary is bundled at `<skills-dir>/repo-inspect/scripts/repo-inspect`. Different agents install skills in different directories. Find it with:

```bash
# Detect skills directory and locate the binary
REPO_INSPECT=""
for dir in ~/.agents/skills ~/.claude/skills ~/.openclaw/skills; do
  if [ -f "$dir/repo-inspect/scripts/repo-inspect" ]; then
    REPO_INSPECT="$dir/repo-inspect/scripts/repo-inspect"
    break
  fi
done

if [ -z "$REPO_INSPECT" ]; then
  echo "ERROR: repo-inspect binary not found. Install the skill: npx skills add https://github.com/gjczone/repo-inspect --skill repo-inspect"
  exit 1
fi
```

**Step 3b: Run the command.** Spawn a **Bash subagent** to execute the binary. The binary writes results to `.inspect/`.

```bash
$REPO_INSPECT --repo . <command> <args> --output md
```

**NEVER** run the binary yourself — use a subagent. The binary may take a few seconds on large repos.

**NEVER** hardcode a single skills directory path (like `~/.agents/skills`). Always use the detection loop above — the binary location depends on where the agent installed the skill.

### Step 4: Read and interpret results

Read the generated file in `.inspect/`. The filename follows the pattern `<command>-<sanitized-query>.md`.

```
Read .inspect/find-how-middleware.md
```

The output contains:
- **Files found**: count of relevant files
- **Per-directory breakdown**: key files in each directory
- **Matching lines**: exact line numbers and content, with ±2 lines context

Use this to answer the user's question. Reference specific file paths and line numbers.

### Step 5: Summarize for the user

Present findings clearly:
- What the feature/pattern is
- Which files implement it (with line references)
- Key code snippets that show the technique
- How it connects to the rest of the codebase

## Reference

### Binary: repo-inspect

The binary is a statically-compiled Rust CLI. Source at the repo root.

```
repo-inspect --repo <repo> <command> [options]

Commands:
  find-how   Search how a feature/technique is implemented
  trace      Trace callers and callees of a symbol
  entries    Find entry points (CLI, HTTP, events, plugins)
  patterns   Detect design patterns and conventions
  data       Extract core data structures and schemas
  hotspots   Identify frequently changed or complex files

Options:
  --repo      Repository: local path (e.g., ".") or remote GitHub (e.g., "owner/repo")
  --output    Output format: json or md (default: md)
  --out-dir   Output directory (default: .inspect)
  --refresh   Force re-fetch remote repo, bypass 24h cache (remote mode only)
  --depth     Search depth for find-how (1-3, default: 2)
```

### Output format

```
.inspect/
├── find-how-<query>.md         # Results of find-how
├── trace-<symbol>.md           # Results of trace
├── entries.md                  # Entry points found
├── patterns.md                 # Design patterns (all categories)
├── patterns-<category>.md      # Design patterns (filtered, e.g. patterns-concurrency.md)
├── data.md                     # Data structures (all)
├── data-<name>.md              # Data structures (filtered, e.g. data-RepoSpec.md)
└── hotspots.md                 # Hotspots identified
```

Each file is compact (typically 20-80 lines), designed for immediate LLM consumption.

### When NOT to use repo-inspect

- General code review → use `code-review` skill
- Documentation generation → use `project-init` skill  
- TDD / writing tests → use `test-driven-development` skill
- The user just wants to read a specific file → `Read` it directly
- `.inspect/` already has relevant results → read them, don't re-run

## Completion

- [ ] `.inspect/` contains output for the user's query
- [ ] Main agent has read the output file(s)
- [ ] Answer references specific file paths and line numbers from the output
- [ ] Binary ran successfully (no errors from subagent)

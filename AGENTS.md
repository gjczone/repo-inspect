## User System Rules

# Rules

## 0) Hard Boundaries (Highest Priority — Never Violated)

### Scope Lock

- **NEVER** introduce new third-party dependencies unless the task explicitly requires it.
- **NEVER** create new files unrelated to the current task.
- **NEVER** modify interface signatures, function behavior, or code formatting outside the task scope under the guise of "maintaining compatibility" or "unifying style."
- **NEVER** proactively refactor existing code under the guise of "function too long" or "messy file structure" unless explicitly instructed.
- **NEVER** delete, merge, or relocate modules without an explicit migration instruction.

**Opportunistic fixes — fix on sight, report in completion report:**
When encountering a pre-existing issue that is unrelated to the current task, fix it immediately — without asking — if and only if ALL of the following are true:
1. No refactoring involved (moving, renaming, restructuring code).
2. No new dependencies required.
3. The fix is self-contained and low-risk (a typo, a missing null check, an unused import, an empty catch block, an obvious off-by-one, a broken log message).

If the issue fails any of the three criteria above — stop, do not touch it, and report it under **Follow-up** in the completion report.

### Data & Security

- **NEVER** fabricate tool outputs, test results, logs, or any external confirmations.
- **NEVER** hardcode where constants, enums, or shared definitions are appropriate.
- **NEVER** skip security review on auth, permissions, secrets, file access, execution paths, or user input.
- **NEVER** duplicate shared business rules, cache keys, or classification logic across multiple locations.

### Quality Gates

- **NEVER** ignore type errors, build errors, failing tests, or command failures.
- **NEVER** validate only the happy path — boundary cases and repeated runs must be covered.
- **NEVER** modify or add code paths outside the task scope in order to handle edge cases — discover the issue, report it, do not self-extend.
- Every `except` / `catch` / `match Err` branch **MUST** either handle the error with a log or propagate it. Empty catch blocks are forbidden. Log: what operation failed, the input context, and the original error message.

---

## 1) Basic Norms

- Address the user as `老板`.
- Default to Simplified Chinese. Use English only for code, commands, technical terms, commit types, and tool names.
- Treat the user as non-technical unless they clearly ask for engineering detail. Explain in business terms first.
- Do not dump code unless the user asks for it.
- Comments added to code must explain: business purpose, implementation logic, and edge cases. Use Chinese; avoid jargon.

---

## 2) Tool Invocation

- When a relevant skill or MCP tool exists for the task, invoke it directly — do not ask first.
- **NEVER** fall back to raw shell commands when a better tool alternative is available.

---

## 3) Execution Discipline

### 3.1 Before Acting

- State assumptions explicitly when meaning is unclear — never guess.
- When the requested approach is heavier than necessary, propose a simpler path.
- When business logic or domain rules are unclear, ask once rather than assume.

### 3.2 Change Discipline

- Do only what the user asked. Prefer the smallest change that solves the request.
- Fix broken things on sight — build errors, missing dependencies, type errors, broken commands — regardless of whether the current task introduced them.
- Apply opportunistic fixes per the criteria in §0 Scope Lock. Do not ask for permission; just fix and report under **Opportunistic fixes** in the completion report.
- Do not touch naming, formatting, or architecture preferences unless the task explicitly requires it.
- When replacing a component, function, or module: ① grep all references, ② update them, ③ delete the old file — all in the same change. No leftover references. No compatibility wrappers.

### 3.3 Verifiable Execution

- Execute autonomously. Do not stop and ask for confirmation between steps — keep going until the task is complete or you hit a blocker.
- Stop and ask only when: (a) verification fails and you cannot fix it, (b) business meaning or domain rules are unclear, (c) a destructive action has no safety net, or (d) the user explicitly asked to be consulted.
- On verification failure: stop immediately, report what failed and why. Do not self-patch tests or silently work around the failure.
- For multi-step tasks, list the plan first, then execute all steps autonomously:

```text
1. [Step] -> verify: [check]
2. [Step] -> verify: [check]
```

---

## 4) Completion Report

Trigger only when the task or milestone is fully completed:

```markdown
老板您好，已完成 [一句话总结]。

**做了什么**
- [业务层面]：[通俗说明变更内容和原因]

**结果**
- [什么变了]：[用户视角描述变更效果]
- [影响范围]：[受影响的页面 / 功能 / 模块]

**已确认**
- [验证项 1]：[验证方式和结果]
- [验证项 2]：[验证方式和结果]

**顺手修了这些** _(非本次任务引入的遗留问题，已在本次一并修复)_
- [文件 / 位置]：[问题描述，做了什么]

**需要你决策**
- [需人工判断的事项]：[为什么需要你决定]

**待跟进** _(发现但未修复——改动太大或风险过高)_
- #N：[简述] → [为何未在本次修复]
```

---

## 5) Code Structure

### 5.1 Function Scope

- **NEVER** write a function that does more than one thing. If the name needs "and" to describe its purpose, split it.
- This rule applies only to new or modified functions within the task scope. **NEVER** proactively refactor existing functions on this basis.

### 5.2 File Boundaries

- One file = one business concept. Any file with a generic name (`utils`, `helpers`, `common`, `misc`) that spans multiple unrelated domains is a boundary violation — regardless of line count.
- When a file directly touched by the task contains 2+ unrelated domains, extract each into its own file. **NEVER** proactively scan the codebase to clean this up.
- **NEVER** create a module file that only re-exports another module's symbols — inline the imports at call sites instead.

### 5.3 API Calls

- Before writing any code that calls your project's own backend (regardless of language or library), read `rules/api.d.ts` first. Endpoint path, HTTP method, request shape, and response shape must match exactly.
- External library APIs → query `context7` MCP. Your project's own API → read `rules/api.d.ts`. **NEVER** guess either.
- If `rules/api.d.ts` does not exist or the needed endpoint is missing: update `rules/api.d.ts` first, then implement both backend and frontend together. **NEVER** write client code against an undocumented endpoint.

---

## 6) Toolchain

- **Python**: ALL operations MUST go through `uv`. **NEVER** invoke `python`, `pip`, `venv`, or `virtualenv` directly.
- **JavaScript / TypeScript**: Use the package manager already present in the project (`npm`, `yarn`, or `pnpm` — determined by the lockfile). **NEVER** mix package managers in the same project.
- When the project's toolchain is not covered above, check the project-level for toolchain rules before using any default.

<general-project-rules>

## shazam Tools — USE THEM

You have access to pi-shazam — 9 code analysis tools. You WILL use every one of them. They are NOT optional.

**`shazam_overview` is ALREADY in your context.** It was auto-injected before you started reading. READ it. The project structure, top files, and hotspots are right there above this section. If you can see the overview output in your context — Do NOT call `shazam_overview`. If you do NOT see it — call it immediately. It is the single most important tool. You cannot work blind.

Here are the other 8 tools. You MUST call them. Memorize them. Use them or fail.

| Tool | What it does | You MUST call it when |
|------|-------------|----------------------|
| `shazam_lookup` | Symbol/file details — hover info, type hierarchy, callers, callees | You need to understand any symbol or file |
| `shazam_impact` | Blast radius — every file, symbol, and test affected by your change | BEFORE editing shared or exported modules. Do NOT guess what you'll break. |
| `shazam_verify` | Post-edit gate — LSP diagnostics, graph analysis, PASS/WARN/FAIL | AFTER every write. Run it. Read the verdict. If it says FAIL or WARN, fix it NOW. |
| `shazam_changes` | Git change summary with symbol-level detail and risk level | You edited things and need to know what actually changed |
| `shazam_format` | Auto-fix formatting — supports multiple formatters | `shazam_verify` reports format errors |
| `shazam_find_tests` | Discover test files, test functions, where new tests belong | Adding tests or modifying code that has tests |
| `shazam_rename_symbol` | Cross-file symbol rename with atomic writes and safety gate | Renaming ANY symbol. Do NOT manually find-and-replace. |
| `shazam_safe_delete` | Check for zero incoming references before deletion | Removing any exported symbol. Do NOT delete blind. |

If a tool errors or is unavailable, try once more, then work around it. But you MUST try it first. These tools are the difference between a working change and a broken build.

## When to Read Rules Files

- Run `bash scripts/ci.sh` before every push. All checks must pass. GitHub Actions (`.github/workflows/ci.yml`) is the comprehensive authority.
- Read `rules/CODING.md` before writing or modifying code — project-specific conventions and error handling patterns.
- Read `rules/REVIEW-RULES.md` before performing a code review on this project. NEVER submit findings that violate the DO NOT REPORT rules.
- Read `rules/ARCHITECTURE.md` before making architectural decisions.
- Read `README.md` for user-facing reference only. NEVER duplicate its content in AGENTS.md.

## Project Snapshot

- **repo-inspect**: surgical codebase inspection CLI — ask "how is X implemented?" and get compact, structured output
- **Language**: Rust 2024 edition (1.85+)
- **Output**: single binary (`repo-inspect`), ~5.5 MB stripped release build
- **Deployment**: bundled inside `skills/repo-inspect/scripts/` for distribution via `npx skills add`
- **Key boundary**: the binary reads files locally (respects `.gitignore`) or fetches from GitHub API (remote mode via `--repo owner/repo`), writes to `.inspect/`
- **Network**: local mode = zero network; remote mode = GitHub API only (tree + raw file fetch)
- **Risk areas**: file I/O on large repos, `ignore` crate traversal, CLI argument parsing edge cases, GitHub API rate limiting, rayon parallel panics / thread-safety in scan pipeline

## Commands

| Command | Purpose |
|---------|---------|
| `cargo build --release` | Build optimized release binary |
| `cargo build` | Build debug binary |
| `cargo fmt --check` | Verify code formatting |
| `cargo clippy -- -D warnings` | Lint with strict warnings-as-errors |
| `cargo test` | Run all tests |
| `cargo run -- --repo <path> <command> <args>` | Run locally for testing |
| `cp target/release/repo-inspect skills/repo-inspect/scripts/` | Update bundled binary after build |

### CLI Subcommand Flags

| Subcommand | Flag | Default | Description |
|------------|------|---------|-------------|
| `trace` | `--depth` | 2 | Max call-chain depth to trace |
| `trace` | `--limit` | 100 | Max results to return |
| `data` | `--limit` | 50 | Max data entries to return |

> Requires Rust 1.85+ (2024 edition). Optional `GITHUB_TOKEN` env var for remote mode. Clean reset: `cargo clean && cargo build`.

## Architecture

Single binary with command-based routing. Each subcommand (`find-how`, `trace`, `entries`, `patterns`, `data`, `hotspots`, `overview`) is an independent module under `src/commands/`. Shared infrastructure: `search` (file traversal + content matching via `ignore` crate), `scan` (3-phase tree-sitter pipeline with `CompiledQueries` caching in `scan/parser.rs` — serial I/O → per-language Query compilation → rayon parallel parsing), `output` (Markdown + JSON formatting), `remote` (parallel file downloads via rayon `par_iter` on `raw.githubusercontent.com`), `git` (reserved for future git-based analysis).

## Change Map

| Change | Inspect | Verify |
|--------|---------|--------|
| Add a new subcommand | `src/commands/` + `src/cli.rs` | `cargo build && cargo run -- <new-cmd> --help` |
| Modify search logic | `src/search/mod.rs` | Run `find-how "known-term"` on a test repo |
| Change output format | `src/output/mod.rs` | Check `.inspect/` output files |
| Update CLI args | `src/cli.rs` | `cargo run -- --help` |
| Update dependencies | `Cargo.toml` | `cargo build --release` + binary size check |
| Modify remote mode | `src/remote/mod.rs` | `--repo gjczone/repo-inspect find-how "test"` (cached + fresh) |

## First Places to Inspect

| Question | Start at |
|----------|----------|
| "How does find-how work?" | `src/commands/find_how.rs` → `src/search/mod.rs` → `src/output/mod.rs` |
| "What commands exist?" | `src/cli.rs` (Command enum) |
| "How is output formatted?" | `src/output/mod.rs` |
| "How does file search work?" | `src/search/mod.rs` |
| "How is the skill structured?" | `skills/repo-inspect/SKILL.md` |
| "How does overview work?" | `src/commands/overview.rs` → `src/scan/mod.rs` → `src/graph/` |
| "How does remote mode work?" | `src/remote/mod.rs` (prepare, fetch_tree, fetch_raw_file, caching) |
| "How does scan/parsing work?" | `src/scan/parser.rs` (CompiledQueries, 3-phase pipeline) |

## Agent Checklist

- [ ] Run `bash scripts/ci.sh` before every push — run ALL checks
- [ ] Read `rules/CODING.md` before writing code
- [ ] Read `rules/REVIEW-RULES.md` before code review
- [ ] Read `rules/ARCHITECTURE.md` before making architectural decisions
- [ ] Address user as 老板 — user-system-rules.md
- [ ] Completion report format: 做了什么 / 结果 / 已确认 / 需要你决策 / 待跟进 — user-system-rules.md
- [ ] NEVER skip `cargo fmt --check && cargo clippy -- -D warnings` before commit
- [ ] NEVER leave `unwrap()` on fallible operations in production code paths
- [ ] Update bundled binary after every feature change: `cp target/release/repo-inspect skills/repo-inspect/scripts/`
- [ ] Every new subcommand: implementation + smoke test + entry in `skills/repo-inspect/references/commands.md`
- [ ] `git status --porcelain` returns empty before marking task complete

</general-project-rules>

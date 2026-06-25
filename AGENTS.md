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

- Before writing any code that calls your project's own backend (regardless of language or library), read `./api.d.ts` first. Endpoint path, HTTP method, request shape, and response shape must match exactly.
- External library APIs → query `context7` MCP. Your project's own API → read `./api.d.ts`. **NEVER** guess either.
- If `api.d.ts` does not exist or the needed endpoint is missing: update `api.d.ts` first, then implement both backend and frontend together. **NEVER** write client code against an undocumented endpoint.

---

## 6) Toolchain

- **Python**: ALL operations MUST go through `uv`. **NEVER** invoke `python`, `pip`, `venv`, or `virtualenv` directly.
- **JavaScript / TypeScript**: Use the package manager already present in the project (`npm`, `yarn`, or `pnpm` — determined by the lockfile). **NEVER** mix package managers in the same project.
- When the project's toolchain is not covered above, check the project-level for toolchain rules before using any default.

<general-project-rules>

## When to Read Rules Files

- Read `rules/LOCAL_CI.md` before every push. Run ALL checks. Failing any = broken commit. GitHub Actions (`.github/workflows/ci.yml`) is the comprehensive authority.
- Read `rules/OPS.md` before any release. Contains build → bundle → tag → release procedures.
- Read `rules/LLM-REVIEW-GUIDE.md` before performing a code review on this project.
- Read `rules/CODING.md` before writing or modifying code.
- Read `rules/TESTING.md` before writing or modifying tests.
- Read `rules/DEBUGGING.md` before debugging issues.
- Read `rules/API-RULES.md` before designing or modifying CLI flags or output formats.
- Read `rules/DATA-STATE.md` before working with data, cache, or file persistence.
- Read `rules/VERIFICATION.md` before marking work complete.
- Read `rules/ERROR-HANDLING.md` before handling errors or boundary cases.
- Read `rules/LOGGING.md` before adding logs or debugging output.
- Read `rules/SECURITY.md` before handling authentication, tokens, or security-sensitive code.
- Read `rules/DEPENDENCIES.md` before adding, updating, or removing dependencies.
- Read `rules/ARCHITECTURE.md` before making architectural decisions.
- Read `rules/PERFORMANCE.md` before optimizing performance or profiling.
- Read `README.md` for user-facing reference only. NEVER duplicate its content in AGENTS.md.
- Read `OPS.md` for the full 7-phase release workflow. NEVER guess release commands.

## Project Snapshot

- **repo-inspect**: surgical codebase inspection CLI — ask "how is X implemented?" and get compact, structured output
- **Language**: Rust 2024 edition (1.85+)
- **Output**: single binary (`repo-inspect`), ~5.5 MB stripped release build
- **Deployment**: bundled inside `skills/repo-inspect/scripts/` for distribution via `npx skills add`
- **Key boundary**: the binary reads files locally (respects `.gitignore`) or fetches from GitHub API (remote mode via `--repo owner/repo`), writes to `.inspect/`
- **Network**: local mode = zero network; remote mode = GitHub API only (tree + raw file fetch)
- **Risk areas**: file I/O on large repos, `ignore` crate traversal, CLI argument parsing edge cases, GitHub API rate limiting

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

## Development Environment

- **Rust**: 1.85+ (2024 edition). Install via `rustup`.
- **Dependencies**: all in `Cargo.toml` — `clap`, `ignore`, `regex`, `serde`/`serde_json`, `walkdir`, `anyhow`, `thiserror`, `log`/`env_logger`, `minreq` (sync HTTP), `tree-sitter` + grammars
- **No external services**, no ports, no env vars required (optional `GITHUB_TOKEN` for remote mode)
- **Clean reset**: `cargo clean && cargo build`

## Architecture

Single binary with command-based routing. Each subcommand (`find-how`, `trace`, `entries`, `patterns`, `data`, `hotspots`) is an independent module under `src/commands/`. Shared infrastructure: `search` (file traversal + content matching via `ignore` crate), `output` (Markdown + JSON formatting), `git` (reserved for future git-based analysis).

## Core Flows

1. **find-how**: CLI args → `FileFinder::walk()` (respects `.gitignore`) → keyword scoring → `extract_matching_lines()` → `OutputWriter::write_markdown()` → `.inspect/` file
2. **Remote mode**: `--repo owner/repo` → `remote::prepare()` → GitHub API tree fetch → raw file download → cache → then same local analysis pipeline
3. **Skill usage**: Agent spawns subagent → subagent runs `scripts/repo-inspect find-how "query"` → binary writes `.inspect/` → main agent reads `.inspect/` file
4. **Build & bundle**: `cargo build --release` → `cp target/release/repo-inspect skills/repo-inspect/scripts/` → commit

## Change Map

| Change | Inspect | Verify |
|--------|---------|--------|
| Add a new subcommand | `src/commands/` + `src/cli.rs` | `cargo build && cargo run -- <new-cmd> --help` |
| Modify search logic | `src/search/mod.rs` | Run `find-how "known-term"` on a test repo |
| Change output format | `src/output/mod.rs` | Check `.inspect/` output files |
| Update CLI args | `src/cli.rs` | `cargo run -- --help` |
| Update dependencies | `Cargo.toml` | `cargo build --release` + binary size check |
| Modify remote mode | `src/remote/mod.rs` | `--repo gjczone/repo-inspect find-how "test"` (cached + fresh) |

## Verification Matrix

| Check | Command | Pass criteria |
|-------|---------|---------------|
| Format | `cargo fmt --check` | No diff |
| Clippy | `cargo clippy -- -D warnings` | Exit 0, zero warnings |
| Build | `cargo build --release` | Exit 0 |
| Test | `cargo test` | Exit 0, 0 failed |
| Binary size | `ls -lh target/release/repo-inspect` | < 6 MB |
| find-how smoke | `cargo run -- --repo . find-how "test" --depth 1` | Exit 0, output in `.inspect/` |
| Remote smoke | `./target/release/repo-inspect --repo gjczone/repo-inspect find-how "test"` | Exit 0 (requires GITHUB_TOKEN) |

## First Places to Inspect

| Question | Start at |
|----------|----------|
| "How does find-how work?" | `src/commands/find_how.rs` → `src/search/mod.rs` → `src/output/mod.rs` |
| "What commands exist?" | `src/cli.rs` (Command enum) |
| "How is output formatted?" | `src/output/mod.rs` |
| "How does file search work?" | `src/search/mod.rs` |
| "How is the skill structured?" | `skills/repo-inspect/SKILL.md` |
| "How does remote mode work?" | `src/remote/mod.rs` (prepare, fetch_tree, fetch_raw_file, caching) |

## Coding Rules

See `rules/CODING.md`. Key anchors: NEVER add deps without justification. NEVER leave `unwrap()` on fallible ops. CLI interface IS the API — never break backward compat.

## Testing Rules

See `rules/TESTING.md`. Key anchors: one test per subcommand minimum. Use `--repo .` as test data. Smoke test every subcommand.

## Debugging Rules

See `rules/DEBUGGING.md`. Key anchors: use `log` crate, not `eprintln!`. Run with `RUST_LOG=debug`. Check stderr for anyhow error chain.

## API Rules

See `rules/API-RULES.md`. Key anchors: CLI = API. Output filenames: `<command>-<sanitized-query>.<ext>`. `--output json` must be valid.

## Data & State Rules

See `rules/DATA-STATE.md`. Key anchors: binary is stateless. `.inspect/` output only. Remote cache: `~/.cache/repo-inspect/remote/`.

## Verification Before Completion

See `rules/VERIFICATION.md`. Run ALL checks: `cargo fmt --check && cargo clippy -- -D warnings && cargo build --release && cargo test` + smoke + binary update + docs sync.

## Agent Checklist

- [ ] Read `rules/LOCAL_CI.md` before every push — run ALL checks
- [ ] Read `rules/VERIFICATION.md` before marking work complete
- [ ] Read `rules/CODING.md` before writing code
- [ ] Read `rules/DEPENDENCIES.md` before adding/updating deps
- [ ] Address user as 老板 — user-system-rules.md
- [ ] Completion report format: 做了什么 / 结果 / 已确认 / 需要你决策 / 待跟进 — user-system-rules.md
- [ ] NEVER skip `cargo fmt --check && cargo clippy -- -D warnings` before commit
- [ ] NEVER leave `unwrap()` on fallible operations in production code paths
- [ ] Update bundled binary after every feature change: `cp target/release/repo-inspect skills/repo-inspect/scripts/`
- [ ] Every new subcommand: implementation + smoke test + entry in `skills/repo-inspect/references/commands.md`
- [ ] `git status --porcelain` returns empty before marking task complete

</general-project-rules>

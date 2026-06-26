# OPS.md — repo-inspect

Release and operational procedures for the repo-inspect CLI tool and skill.

---

## 0) Hard Boundaries

- **NEVER** start a release when main branch CI is red — fix it first. A release from a broken main is a broken release.
- **NEVER** release without updating the bundled binary in `skills/repo-inspect/scripts/repo-inspect`.
- **NEVER** merge and walk away — after merge, wait for main CI to go green. If it's red, fix it immediately.
- **NEVER** skip Phase 7 (retrospective) — defer is forbidden.

---

## Phase 0: Verify Main Branch CI is Green

**Before starting any release work**, the main branch CI MUST be green.

```bash
gh run list --branch main --limit 5 --json status,conclusion,name,databaseId
```

**Pass**: ALL workflows on main show `conclusion: SUCCESS`. If any check is red: STOP. Fix the red CI first — create a `fix/` branch, go through the full pipeline, merge, wait for main CI to go green, then return here.

---

## Phase 1: Documentation Sync

Before bumping the version, cross-check all changed files against companion files:

| File | Check |
|------|-------|
| `README.md` | Install instructions, command list, output examples match current CLI |
| `AGENTS.md` | Commands, Verification Matrix, Change Map reflect current state |
| `skills/repo-inspect/SKILL.md` | Workflow steps match current binary behavior |
| `skills/repo-inspect/references/commands.md` | All subcommands documented, flags match `src/cli.rs` |

**Pass**: every changed file cross-checked against companion files. No doc references a command or flag that doesn't exist.

---

## Phase 2: Version Bump & Build

```bash
# Bump version in Cargo.toml
# (edit the version field manually)

# Build release binary
cargo build --release
```

**Pass**: `cargo build --release` exits 0. Binary at `target/release/repo-inspect`.

---

## Phase 3: Local CI

Run the full `rules/LOCAL_CI.md` checklist:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo build --release
cargo test
./target/release/repo-inspect --repo . find-how "search" --depth 1
```

**Pass**: all 5 steps green, 0 failures.

---

## Phase 4: GitHub Release

### 4a: Bundle the binary

```bash
cp target/release/repo-inspect skills/repo-inspect/scripts/repo-inspect
```

### 4b: Create tag and release

```bash
VERSION="v$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
git tag "$VERSION"
git push origin "$VERSION"

gh release create "$VERSION" \
  --title "repo-inspect $VERSION" \
  --notes "$(cat <<'EOF'
## What's Changed

- INSERT: summary of changes in this release

## Install

```bash
npx skills add https://github.com/gjczone/repo-inspect --skill repo-inspect
```

## Binary

The pre-built Linux x86_64 binary is bundled in `skills/repo-inspect/scripts/repo-inspect`.
EOF
)" \
  skills/repo-inspect/scripts/repo-inspect
```

**Pass**: `gh release view "$VERSION"` shows the release with binary attached. Release notes contain "What's Changed" and install instructions — not just "See CHANGELOG."

---

## Phase 5: Post-Release Verification

```bash
# Verify the binary runs
./skills/repo-inspect/scripts/repo-inspect --version

# Smoke test against this repo
./skills/repo-inspect/scripts/repo-inspect --repo . find-how "search" --depth 1
```

**Pass**: `--version` prints the correct version. Smoke test exits 0 and creates `.inspect/` output.

---

## Phase 6: Cleanup

```bash
git push origin --delete <release-branch> 2>/dev/null || true
```

**Pass**: working directory clean (`git status --porcelain` returns empty). On main branch.

---

## Phase 7: Self-Improvement Retrospective

### 7.1 Companion File Audit

| File | Check |
|------|-------|
| `README.md` | Feature descriptions, install commands match current release |
| `AGENTS.md` | Commands, rules, verification matrix still valid |
| `rules/LOCAL_CI.md` | All CI steps still valid — any new checks to add? |
| `rules/OPS.md` | This file — any steps missing, wrong order, or weak Pass criteria? |
| `skills/repo-inspect/SKILL.md` | Workflow, command table, binary detection still accurate |
| `skills/repo-inspect/references/commands.md` | All subcommands documented |
| `Cargo.toml` | Dependencies current, no unused crates |

**Pass**: every file opened and checked. Any file documenting something changed in this release has been updated.

### 7.2 Process Retrospective

- Were there manual steps NOT documented in this file? → Add them now.
- Did any companion file go stale and only get caught late? → Add a check to Phase 1.
- Did any Pass criteria fail to catch a real problem? → Strengthen the criteria now.
- Were there "I forgot to do X" moments? → Add a checklist item now.

**Pass**: all "yes" answers resolved with a concrete change committed in this release.

---

## Release Checklist (Summary)

0. [ ] Main branch CI verified green (Phase 0)
1. [ ] Documentation sync completed (Phase 1)
2. [ ] Version bump + build succeeded (Phase 2)
3. [ ] Local CI passed — 0 failures (Phase 3)
4. [ ] Release published with binary attached (Phase 4)
5. [ ] Post-release verification passed (Phase 5)
6. [ ] Temporary branches deleted, working directory clean (Phase 6)
7. [ ] Companion file audit completed (Phase 7.1)
8. [ ] Process retrospective completed — all "yes" answers resolved (Phase 7.2)

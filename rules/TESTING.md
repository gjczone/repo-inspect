# Testing Rules — repo-inspect

Rust tests via `cargo test`. All rules come from evidence in the repository.

## Test Tiers

| Tier | What it covers | Required when |
|------|---------------|---------------|
| **Unit** | Individual functions, methods, modules | Always |
| **Smoke** | `cargo run -- --repo . find-how "test" --depth 1` exits 0 | Always |
| **Integration** | Cross-module interaction (remote + search, scan + graph) | New module crosses boundary |

## Test Discipline

- **NEVER** merge a PR without a test for the new/changed behavior. One test per subcommand is the minimum.
- **NEVER** skip `cargo test` before pushing — all tests MUST pass.
- **NEVER** write a test that depends on a specific real repository being present — use the current project itself (`--repo .`) as test data.
- Smoke test for every subcommand: run against `--repo .` and verify exit code 0.

## Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_owner_repo_valid() {
        let (owner, repo) = parse_owner_repo("gjczone/repo-inspect").unwrap();
        assert_eq!(owner, "gjczone");
        assert_eq!(repo, "repo-inspect");
    }

    #[test]
    fn test_parse_owner_repo_invalid() {
        assert!(parse_owner_repo("invalid").is_err());
    }
}
```

## Test Naming

- Pattern: `test_<function>_<scenario>` (e.g., `test_is_source_file_rust`, `test_parse_owner_repo_empty_owner`)
- Test both happy path AND error paths
- One test = one scenario — **NEVER** combine multiple scenarios in one test

## Coverage Requirements

- New module: at least 1 test per public function
- New subcommand: smoke test + unit test
- Error paths: every `bail!` / `Err` return path must have a test

## Running Tests

```bash
# Full test suite
cargo test                            # 0 failures required

# Specific module
cargo test remote::tests              # verify after changes

# With debug logging
RUST_LOG=debug cargo test            # when test fails unexpectedly
```

## What NOT to Test

- Standard library behavior
- External crate internals (test your integration, not theirs)
- Trivial getters/setters
- `main()` function directly (test via smoke test of the binary)

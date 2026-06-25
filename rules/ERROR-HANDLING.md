# Error Handling Rules — repo-inspect

## Error Types

| Crate | Purpose | Usage |
|-------|---------|-------|
| `anyhow` | Application-level errors with context | `anyhow::Result<T>`, `.context()`, `bail!` |
| `thiserror` | Library-level structured errors | `#[derive(Error)]` on enum variants |

## Rules

- Every error path **MUST** either handle the error (log + recover) or propagate it (via `?`). **NEVER** silently swallow.
- Use `.context()` to add operation-specific information: `fs::read(path).context("Failed to read config")?`
- Log format: what operation failed, the input context, the original error message.
- **NEVER** use `unwrap()` or `expect()` on operations that can fail under normal conditions (I/O, network, parsing).
- Use `bail!` for early returns with clear messages: `bail!("Repository path does not exist: {}", path.display())`

## Pattern

```rust
fn fetch_file(path: &str) -> anyhow::Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))?;
    
    if content.is_empty() {
        bail!("File is empty: {}", path);
    }
    
    Ok(content)
}
```

## Error Messages

- Include the failed input value (e.g., the path, the URL, the repo name)
- For remote errors: distinguish 401 (bad token), 403 (rate limit), 404 (not found)
- Chinese error messages for CLI output, English for internal logs

## Anti-Patterns

| Anti-Pattern | Example | Fix |
|--------------|---------|-----|
| Empty catch | `_ => {}` | Log and return error |
| Bare unwrap | `fs::read(path).unwrap()` | Use `?` with `.context()` |
| Silent fallback | Return default on error without log | Log the error before falling back |
| Vague message | `bail!("error")` | `bail!("Failed to fetch {}: {}", owner, repo)` |

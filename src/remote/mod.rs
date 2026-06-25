//! Remote mode: inspect GitHub repositories without cloning.
//!
//! Uses the GitHub REST API to fetch repository metadata, file tree, and raw file
//! contents.  Results are cached under `~/.cache/repo-inspect/remote/` with a 24-hour
//! TTL so repeated inspections are fast.
//!
//! # Environment
//!
//! - `GITHUB_TOKEN` — personal access token for authentication (optional but
//!   strongly recommended to avoid unauthenticated rate limits of 60 req/h).
//!
//! # Caching
//!
//! | File               | Purpose                                |
//! |--------------------|----------------------------------------|
//! | `meta.json`        | fetch timestamp, branch, file count    |
//! | `<path>`           | raw file content (repo-relative path)  |
//!
//! Pass `--refresh` to force a re-fetch even when the cache is still fresh.

use anyhow::{Context, bail};
use log::{debug, info};
use rayon::prelude::*;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ─── constants ──────────────────────────────────────────────────────────────

/// GitHub REST API base URL.
const API_BASE: &str = "https://api.github.com";

/// Raw content base URL (does not count against API rate limits).
const RAW_BASE: &str = "https://raw.githubusercontent.com";

/// Cache TTL: 24 hours.
const CACHE_TTL: Duration = Duration::from_secs(86_400);

/// Maximum files to fetch from a single repo (safety limit).
const MAX_FILES: usize = 5_000;

/// User-Agent header required by GitHub API.
const USER_AGENT: &str = "repo-inspect/0.1.0";

// ─── API response types (minimal – only the fields we need) ─────────────────

#[derive(Deserialize)]
struct RepoInfo {
    default_branch: String,
}

#[derive(Deserialize)]
struct TreeResponse {
    tree: Vec<TreeItem>,
    #[allow(dead_code)]
    truncated: bool,
}

#[derive(Deserialize)]
struct TreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
}

#[derive(Deserialize)]
struct RateLimitResponse {
    message: Option<String>,
}

// ─── cache metadata ─────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheMeta {
    fetched_at: u64,
    branch: String,
    file_count: usize,
}

// ─── public API ─────────────────────────────────────────────────────────────

/// Parse `owner/repo` into its two parts.
///
/// Returns an error on malformed input (missing `/`, empty parts, too many `/`).
#[allow(dead_code)]
pub fn parse_owner_repo(spec: &str) -> anyhow::Result<(String, String)> {
    let parts: Vec<&str> = spec.split('/').collect();
    if parts.len() != 2 {
        bail!(
            "Invalid remote repo spec \"{}\". Expected format: owner/repo (e.g., gjczone/repo-inspect)",
            spec
        );
    }
    let owner = parts[0].trim();
    let repo = parts[1].trim();
    if owner.is_empty() || repo.is_empty() {
        bail!(
            "Invalid remote repo spec \"{}\". Both owner and repo must be non-empty.",
            spec
        );
    }
    Ok((owner.to_string(), repo.to_string()))
}

/// Prepare a local file tree for the given remote repository.
///
/// 1. Checks the cache — returns the cached directory if fresh and `refresh` is false.
/// 2. Fetches the repository metadata (default branch) from GitHub API.
/// 3. Fetches the full file tree.
/// 4. Filters to source files only.
/// 5. Downloads each file and writes it to the cache directory.
/// 6. Writes `meta.json` and returns the cache directory path.
pub fn prepare(owner: &str, repo: &str, refresh: bool) -> anyhow::Result<PathBuf> {
    let cache_dir = cache_dir_path(owner, repo);

    // 缓存命中（未要求刷新 且 TTL 未过期）
    if !refresh && let Some(dir) = check_cache(&cache_dir) {
        info!(
            "Using cached remote files for {}/{}, cache dir: {}",
            owner,
            repo,
            dir.display()
        );
        eprintln!(
            "Using cached files for {}/{} → {}",
            owner,
            repo,
            dir.display()
        );
        return Ok(dir);
    }

    // 获取 GitHub token（可选）
    let token = std::env::var("GITHUB_TOKEN").ok();

    eprintln!("Fetching {}/{} from GitHub API...", owner, repo);

    // 1. 获取默认分支
    let branch = get_default_branch(owner, repo, token.as_deref())?;
    debug!("Default branch for {}/{}: {}", owner, repo, branch);

    // 2. 获取文件树
    let tree = get_file_tree(owner, repo, &branch, token.as_deref())?;
    debug!("Fetched tree: {} entries", tree.len());

    // 3. 过滤到源文件
    let source_files: Vec<&str> = tree
        .iter()
        .filter(|item| item.item_type == "blob" && is_source_file(&item.path))
        .map(|item| item.path.as_str())
        .take(MAX_FILES)
        .collect();

    if source_files.is_empty() {
        bail!(
            "No source files found in {}/{}. The repository may be empty or contain only non-code files.",
            owner,
            repo
        );
    }

    eprintln!(
        "Downloading {} source files for {}/{}...",
        source_files.len(),
        owner,
        repo
    );

    // 4. 清理旧缓存并创建目录
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)
            .with_context(|| format!("Failed to clear cache directory: {}", cache_dir.display()))?;
    }
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("Failed to create cache directory: {}", cache_dir.display()))?;

    // 5. 并行下载文件（rayon 线程池，raw.githubusercontent.com 不计入 API 限额）
    let fetched = AtomicUsize::new(0);
    let errors = AtomicUsize::new(0);

    // 并行下载，收集成功的结果
    let results: Vec<(&str, String)> = source_files
        .par_iter()
        .filter_map(|rel_path| {
            match fetch_raw_file(owner, repo, &branch, rel_path, token.as_deref()) {
                Ok(content) => {
                    fetched.fetch_add(1, Ordering::Relaxed);
                    Some((*rel_path, content))
                }
                Err(e) => {
                    debug!("Failed to fetch {}: {}", rel_path, e);
                    errors.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        })
        .collect();

    // 串行写入磁盘（避免并发文件系统竞争）
    for (rel_path, content) in &results {
        let dest = cache_dir.join(rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, content)
            .with_context(|| format!("Failed to write cached file: {}", dest.display()))?;
    }

    let fetched = fetched.load(Ordering::Relaxed);
    let errors = errors.load(Ordering::Relaxed);

    if fetched == 0 {
        bail!(
            "Failed to download any source files from {}/{}. {} fetch errors.",
            owner,
            repo,
            errors
        );
    }

    debug!(
        "Fetched {} files, {} errors for {}/{}",
        fetched, errors, owner, repo
    );

    // 6. 写入缓存元数据
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let meta = CacheMeta {
        fetched_at: now,
        branch: branch.clone(),
        file_count: fetched,
    };

    let meta_json =
        serde_json::to_string_pretty(&meta).context("Failed to serialize cache metadata")?;
    fs::write(cache_dir.join("meta.json"), meta_json).context("Failed to write cache metadata")?;

    eprintln!(
        "Fetched {} files for {}/{} → {}",
        fetched,
        owner,
        repo,
        cache_dir.display()
    );

    Ok(cache_dir)
}

// ─── cache helpers ──────────────────────────────────────────────────────────

/// Compute the cache directory path for a given owner/repo.
fn cache_dir_path(owner: &str, repo: &str) -> PathBuf {
    let home = dirs_fallback();
    home.join(".cache")
        .join("repo-inspect")
        .join("remote")
        .join(format!("{}-{}", owner, repo))
}

/// Check whether a cached directory is still fresh.
///
/// Returns `Some(path)` if the cache exists and is within TTL, `None` otherwise.
fn check_cache(dir: &Path) -> Option<PathBuf> {
    let meta_path = dir.join("meta.json");
    if !meta_path.exists() {
        return None;
    }

    let meta: CacheMeta = serde_json::from_str(&fs::read_to_string(&meta_path).ok()?).ok()?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // 检查 TTL
    if now.saturating_sub(meta.fetched_at) > CACHE_TTL.as_secs() {
        debug!(
            "Cache expired for {} (age: {}s, ttl: {}s)",
            dir.display(),
            now - meta.fetched_at,
            CACHE_TTL.as_secs()
        );
        return None;
    }

    // 确保至少有一些文件
    if meta.file_count == 0 {
        return None;
    }

    Some(dir.to_path_buf())
}

// ─── GitHub API helpers ─────────────────────────────────────────────────────

/// Fetch the default branch for a repository.
fn get_default_branch(owner: &str, repo: &str, token: Option<&str>) -> anyhow::Result<String> {
    let url = format!("{}/repos/{}/{}", API_BASE, owner, repo);
    let resp = api_get(&url, token).with_context(|| {
        format!(
            "Failed to fetch repository info for {}/{}. Does the repo exist and is it public?",
            owner, repo
        )
    })?;

    let repo_info: RepoInfo =
        serde_json::from_str(&resp).context("Failed to parse GitHub repo API response")?;

    Ok(repo_info.default_branch)
}

/// Fetch the full recursive file tree for a repository.
fn get_file_tree(
    owner: &str,
    repo: &str,
    branch: &str,
    token: Option<&str>,
) -> anyhow::Result<Vec<TreeItem>> {
    let url = format!(
        "{}/repos/{}/{}/git/trees/{}?recursive=1",
        API_BASE, owner, repo, branch
    );

    let resp = api_get(&url, token)
        .with_context(|| format!("Failed to fetch file tree for {}/{}", owner, repo))?;

    let tree: TreeResponse =
        serde_json::from_str(&resp).context("Failed to parse GitHub tree API response")?;

    if tree.truncated {
        debug!(
            "Warning: tree response was truncated for {}/{} — GitHub API limits exceeded",
            owner, repo
        );
    }

    Ok(tree.tree)
}

/// Fetch the raw content of a single file from a repository.
///
/// Uses `raw.githubusercontent.com` which does not count against API rate limits.
/// Falls back to the authenticated Contents API if the raw URL returns 404.
fn fetch_raw_file(
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    token: Option<&str>,
) -> anyhow::Result<String> {
    // 首选：raw.githubusercontent.com（不计入 API 速率限制）
    let raw_url = format!("{}/{}/{}/{}/{}", RAW_BASE, owner, repo, branch, path);

    match http_get_raw(&raw_url, token) {
        Ok(body) => return Ok(body),
        Err(e) => {
            debug!("raw URL failed for {}: {}", path, e);
        }
    }

    // 回退：GitHub Contents API（需要认证以获取更高限额）
    let api_url = format!(
        "{}/repos/{}/{}/contents/{}?ref={}",
        API_BASE, owner, repo, path, branch
    );

    let resp_body = api_get_with_accept(&api_url, token, "application/vnd.github.v3.raw")
        .with_context(|| format!("Failed to fetch file content for {}", path))?;

    Ok(resp_body)
}

/// Simple GET request returning the response body, or an error for non-200/404.
/// Returns the body as String on 200, returns Err on 404 or other failures.
fn http_get_raw(url: &str, token: Option<&str>) -> anyhow::Result<String> {
    let mut req = minreq::get(url).with_header("User-Agent", USER_AGENT);
    if let Some(t) = token {
        req = req.with_header("Authorization", format!("Bearer {}", t));
    }
    let resp = req
        .send()
        .with_context(|| format!("HTTP request failed: GET {}", sanitize_url(url)))?;
    match resp.status_code {
        200 => Ok(resp
            .as_str()
            .context("Failed to read response body")?
            .to_string()),
        404 => bail!("File not found (404)"),
        other => bail!("HTTP {}", other),
    }
}

/// Perform an authenticated (or unauthenticated) GET request against the GitHub API.
///
/// Returns the response body as a String.
fn api_get(url: &str, token: Option<&str>) -> anyhow::Result<String> {
    api_get_with_accept(url, token, "application/vnd.github+json")
}

/// Perform a GET request with a custom Accept header.
///
/// Handles authentication, rate-limiting, and error responses.
fn api_get_with_accept(url: &str, token: Option<&str>, accept: &str) -> anyhow::Result<String> {
    let mut req = minreq::get(url)
        .with_header("User-Agent", USER_AGENT)
        .with_header("Accept", accept)
        .with_header("Accept-Encoding", "identity");

    if let Some(t) = token {
        req = req.with_header("Authorization", format!("Bearer {}", t));
    }

    let resp = req
        .send()
        .with_context(|| format!("HTTP request failed: GET {}", sanitize_url(url)))?;

    let status = resp.status_code;

    match status {
        200 => {
            let body = resp.as_str().context("Failed to read response body")?;
            Ok(body.to_string())
        }
        401 => {
            let body_str = resp.as_str().unwrap_or("");
            if let Ok(rate) = serde_json::from_str::<RateLimitResponse>(body_str)
                && let Some(msg) = rate.message
            {
                bail!("GitHub API authentication failed: {}", msg);
            }
            bail!(
                "GitHub API returned 401 Unauthorized. Set GITHUB_TOKEN environment variable with a valid token, or check that the repository is public."
            );
        }
        403 => {
            // 通常是速率限制
            let body_str = resp.as_str().unwrap_or("");
            if body_str.contains("rate limit") || body_str.contains("secondary rate limit") {
                bail!(
                    "GitHub API rate limit exceeded. Set GITHUB_TOKEN to increase limits, or wait and try again. Details: {}",
                    body_str
                );
            }
            bail!("GitHub API returned 403 Forbidden: {}", body_str);
        }
        404 => {
            bail!(
                "Repository or resource not found (404). Check that the owner/repo is correct and public."
            );
        }
        other => {
            let body = resp.as_str().unwrap_or("");
            bail!(
                "GitHub API returned HTTP {} for {}: {}",
                other,
                sanitize_url(url),
                body
            );
        }
    }
}

/// Redact the URL of any token-bearing query parameters for safe logging.
fn sanitize_url(url: &str) -> String {
    // GitHub API URLs don't use query params for auth, but safety first.
    // Truncate at 80 chars for readability.
    if url.len() <= 80 {
        url.to_string()
    } else {
        format!("{}...", &url[..77])
    }
}

// ─── source file detection ──────────────────────────────────────────────────

/// Check whether a path looks like a source file (not binary, asset, or generated).
///
/// Mirrors the logic in `src/search/mod.rs` but operates on path strings rather
/// than the filesystem.
fn is_source_file(path: &str) -> bool {
    // 跳过常见非源代码目录
    let lower = path.to_lowercase();
    if lower.contains("node_modules/")
        || lower.contains(".git/")
        || lower.contains("target/")
        || lower.contains("dist/")
        || lower.contains("build/")
        || lower.contains("__pycache__/")
        || lower.contains(".venv/")
        || lower.contains("vendor/")
    {
        return false;
    }

    // 按扩展名判断
    let p = Path::new(path);
    matches!(
        p.extension().and_then(|e| e.to_str()),
        Some(
            "rs" | "py"
                | "js"
                | "ts"
                | "tsx"
                | "jsx"
                | "go"
                | "java"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "rb"
                | "php"
                | "swift"
                | "kt"
                | "scala"
                | "cs"
                | "fs"
                | "vue"
                | "svelte"
                | "json"
                | "yaml"
                | "yml"
                | "toml"
                | "md"
                | "css"
                | "scss"
                | "less"
                | "html"
                | "xml"
                | "sql"
                | "graphql"
                | "proto"
                | "prisma"
                | "r"
                | "jl"
                | "ex"
                | "exs"
                | "erl"
                | "hrl"
                | "dart"
        )
    )
}

/// Find the user's home directory, falling back to `/tmp` if unavailable.
fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

// ─── tests ──────────────────────────────────────────────────────────────────

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
    fn test_parse_owner_repo_with_dashes() {
        let (owner, repo) = parse_owner_repo("my-org/my-repo").unwrap();
        assert_eq!(owner, "my-org");
        assert_eq!(repo, "my-repo");
    }

    #[test]
    fn test_parse_owner_repo_invalid_no_slash() {
        assert!(parse_owner_repo("invalid").is_err());
    }

    #[test]
    fn test_parse_owner_repo_invalid_too_many_slashes() {
        assert!(parse_owner_repo("a/b/c").is_err());
    }

    #[test]
    fn test_parse_owner_repo_empty_owner() {
        assert!(parse_owner_repo("/repo").is_err());
    }

    #[test]
    fn test_parse_owner_repo_empty_repo() {
        assert!(parse_owner_repo("owner/").is_err());
    }

    #[test]
    fn test_is_source_file_rust() {
        assert!(is_source_file("src/main.rs"));
    }

    #[test]
    fn test_is_source_file_python() {
        assert!(is_source_file("app/views.py"));
    }

    #[test]
    fn test_is_source_file_not_source() {
        assert!(!is_source_file("image.png"));
        assert!(!is_source_file("archive.tar.gz"));
        assert!(!is_source_file("Makefile"));
    }

    #[test]
    fn test_is_source_file_skips_node_modules() {
        assert!(!is_source_file("node_modules/foo/index.js"));
    }

    #[test]
    fn test_cache_dir_path() {
        let dir = cache_dir_path("owner", "repo");
        assert!(dir.ends_with("owner-repo"));
    }

    #[test]
    fn test_check_cache_nonexistent() {
        let dir = PathBuf::from("/tmp/nonexistent-cache-test-dir");
        assert!(check_cache(&dir).is_none());
    }
}

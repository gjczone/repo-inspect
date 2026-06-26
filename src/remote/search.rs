//! GitHub Search API integration.
//!
//! Uses the REST Search API (`/search/code`) to find files matching a query
//! within a specific repository. Rate limited to 30 req/min (authenticated)
//! or 10 req/min (unauthenticated).

use anyhow::{Context, bail};
use log::debug;
use serde::Deserialize;

const API_BASE: &str = "https://api.github.com";
const USER_AGENT: &str = "repo-inspect/0.1.0";

// ─── API response types ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SearchResponse {
    items: Vec<SearchItem>,
    total_count: usize,
}

#[derive(Deserialize)]
struct SearchItem {
    path: String,
    name: String,
    #[allow(dead_code)]
    text_matches: Option<Vec<TextMatch>>,
}

#[derive(Deserialize)]
struct TextMatch {
    fragment: String,
}

// ─── public types ────────────────────────────────────────────────────────────

/// A single search result from the GitHub Search API.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Full path of the matching file (relative to repo root).
    pub path: String,
    /// File name only.
    #[allow(dead_code)]
    pub name: String,
    /// Matching code snippet (first match only, for context).
    #[allow(dead_code)]
    pub snippet: String,
}

// ─── public API ──────────────────────────────────────────────────────────────

/// Search for code in a specific GitHub repository.
///
/// Queries the GitHub Search API (`/search/code`) with the format:
/// `{query} repo:{owner}/{repo}`.
///
/// Returns up to 20 matching files. Falls back gracefully on rate limits
/// by returning partial results or an empty list.
pub fn search_code(
    owner: &str,
    repo: &str,
    query: &str,
    token: Option<&str>,
) -> anyhow::Result<Vec<SearchResult>> {
    // 构建搜索查询：关键词 + repo 过滤
    let search_query = format!("{} repo:{}/{}", query, owner, repo);
    let encoded = url_encode(&search_query);

    let url = format!("{}/search/code?q={}&per_page=20", API_BASE, encoded);

    debug!("Search API request: {}", url);

    let resp_body = api_get_search(&url, token).with_context(|| {
        format!(
            "GitHub Search API failed for query \"{}\" in {}/{}",
            query, owner, repo
        )
    })?;

    let response: SearchResponse = serde_json::from_str(&resp_body).with_context(|| {
        format!(
            "Failed to parse Search API response for query \"{}\". Response length: {} chars",
            query,
            resp_body.len()
        )
    })?;

    debug!(
        "Search API: {} total results, returning {} items",
        response.total_count,
        response.items.len()
    );

    if response.items.is_empty() {
        return Ok(Vec::new());
    }

    let results: Vec<SearchResult> = response
        .items
        .into_iter()
        .map(|item| {
            let snippet = item
                .text_matches
                .as_ref()
                .and_then(|matches| matches.first())
                .map(|m| m.fragment.clone())
                .unwrap_or_default();

            SearchResult {
                path: item.path,
                name: item.name,
                snippet,
            }
        })
        .collect();

    Ok(results)
}

// ─── HTTP helpers ────────────────────────────────────────────────────────────

/// Perform a GET request against the GitHub Search API.
///
/// Handles rate limiting specially: returns empty results on 403 (rate limit)
/// instead of erroring, so callers can fall back to full download.
fn api_get_search(url: &str, token: Option<&str>) -> anyhow::Result<String> {
    let mut req = minreq::get(url)
        .with_header("User-Agent", USER_AGENT)
        .with_header("Accept", "application/vnd.github.v3.text-match+json")
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
        403 => {
            // Search API 速率限制 — 返回空结果而不是错误
            let body_str = resp.as_str().unwrap_or("");
            if body_str.contains("rate limit") || body_str.contains("secondary rate limit") {
                debug!("Search API rate limited, returning empty results");
                // 返回空 JSON 数组，让调用方降级
                Ok(r#"{"total_count":0,"items":[]}"#.to_string())
            } else {
                bail!("GitHub API returned 403 Forbidden: {}", body_str);
            }
        }
        422 => {
            // 查询语法错误或仓库太小（没有搜索索引）
            debug!("Search API 422 — query may be malformed or repo has no search index");
            Ok(r#"{"total_count":0,"items":[]}"#.to_string())
        }
        401 => {
            bail!(
                "GitHub API authentication failed (401). Set GITHUB_TOKEN or check that the repository is public."
            );
        }
        404 => {
            bail!("Repository not found (404). Check that the owner/repo is correct and public.");
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

/// Manually URL-encode a string for use in query parameters.
fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push(hex_char(byte >> 4));
                result.push(hex_char(byte & 0x0F));
            }
        }
    }
    result
}

fn hex_char(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'A' + (n - 10)) as char,
        _ => '0',
    }
}

/// Redact the URL for safe logging.
fn sanitize_url(url: &str) -> String {
    if url.len() <= 80 {
        url.to_string()
    } else {
        format!("{}...", &url[..77])
    }
}

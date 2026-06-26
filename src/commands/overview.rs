//! `overview` subcommand — single-command project spine.
//!
//! Provides a holistic project overview: language breakdown, dependencies, recent
//! changes, module structure, top files by PageRank, entry points, complexity
//! hotspots, and suggested reading order.
//!
//! Supports two modes:
//! - **Full mode** (local repo or `--full` remote): tree-sitter scan + graph +
//!   PageRank for detailed symbol-level analysis.
//! - **Lightweight mode** (remote without `--full`): file-tree metadata +
//!   API-fetched config/commits — no source file downloads, ~2 seconds.

use crate::cli::{OutputFormat, OverviewArgs};
use crate::output::OutputWriter;
use anyhow::{Context, Result};
use log::debug;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ─── output data structures ──────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct OverviewOutput {
    project_name: String,
    symbol_count: usize,
    file_count: usize,
    language_count: usize,
    is_lightweight: bool,
    languages: Vec<LanguageStat>,
    dependencies: Vec<DependencyInfo>,
    recent_changes: Vec<ChangeEntry>,
    top_files: Vec<FileRankInfo>,
    entry_points: Vec<EntryPointInfo>,
    module_structure: Vec<DirInfo>,
    hotspots: Vec<HotspotInfo>,
    reading_order: Vec<String>,
}

#[derive(serde::Serialize)]
struct LanguageStat {
    language: String,
    file_count: usize,
}

#[derive(serde::Serialize)]
struct DependencyInfo {
    name: String,
    version: String,
}

#[derive(serde::Serialize)]
struct ChangeEntry {
    sha: String,
    message: String,
}

#[derive(serde::Serialize)]
struct FileRankInfo {
    rank: usize,
    file: String,
    symbols: usize,
    pagerank: f64,
    top_symbol: String,
}

#[derive(serde::Serialize)]
struct EntryPointInfo {
    kind: String,
    name: String,
    file: String,
    line: usize,
    pagerank: f64,
}

#[derive(serde::Serialize)]
struct DirInfo {
    path: String,
    file_count: usize,
    children: Vec<DirInfo>,
}

#[derive(serde::Serialize)]
struct HotspotInfo {
    rank: usize,
    file: String,
    score: f64,
    symbols: usize,
    pagerank: f64,
}

// ─── public entry point ──────────────────────────────────────────────────────

/// Run the overview command.
///
/// Detects whether we're in lightweight mode (remote Tier 1 with `tree.json`)
/// or full mode (local files available for tree-sitter scanning).
pub fn run(args: OverviewArgs, repo: &Path, out_dir: &Path, format: OutputFormat) -> Result<()> {
    let query = args.filter.as_deref().unwrap_or("");
    let writer = OutputWriter::new(out_dir, "overview", query, format)?;

    let is_lightweight = repo.join("tree.json").exists();

    let data = if is_lightweight {
        debug!("Overview: lightweight mode (tree.json detected)");
        build_lightweight_overview(repo, args.filter.as_deref())?
    } else {
        debug!("Overview: full mode (tree-sitter scan + graph + PageRank)");
        build_full_overview(repo, args.filter.as_deref())?
    };

    match format {
        OutputFormat::Json => write_json(&writer, &data)?,
        OutputFormat::Md => write_markdown(&writer, &data)?,
    }

    let proj = &data.project_name;
    let syms = data.symbol_count;
    let files = data.file_count;
    let langs = data.language_count;
    let mode = if data.is_lightweight {
        "轻量级"
    } else {
        "完整分析"
    };
    eprintln!(
        "Overview ({mode}): {proj} — {syms} symbols, {files} files, {langs} languages → {}",
        writer.output_file().display()
    );

    Ok(())
}

// ─── full mode (local repo / --full remote) ──────────────────────────────────

/// Build overview data from a full tree-sitter scan + graph + PageRank pipeline.
fn build_full_overview(repo: &Path, filter: Option<&str>) -> Result<OverviewOutput> {
    let project_name = project_name_from_path(repo);

    // 1. Scan project for symbols
    let scan_result = crate::scan::scan_project(repo)?;
    let file_count = scan_result.files.len();
    let symbol_count = scan_result.symbol_count;

    // 2. Build graph and calculate PageRank
    let mut graph = crate::graph::builder::build_graph(&scan_result);
    crate::graph::pagerank::calculate_pagerank_default(&mut graph);

    // 3. Collect data
    let languages = collect_language_stats(&scan_result);
    let dependencies = collect_dependencies(repo)?;
    let recent_changes = collect_recent_changes(repo)?;
    let file_pageranks = compute_file_pageranks(&graph);
    let top_files = build_top_files(&file_pageranks, &graph, 10, filter);
    let entry_points = find_entry_points(&graph, filter);
    let module_structure = compute_module_structure_from_scan(&scan_result, filter);
    let hotspots = compute_file_hotspots(&graph, &file_pageranks, filter);
    let reading_order = suggest_reading_order(&file_pageranks, filter);

    Ok(OverviewOutput {
        project_name,
        symbol_count,
        file_count,
        language_count: languages.len(),
        is_lightweight: false,
        languages,
        dependencies,
        recent_changes,
        top_files,
        entry_points,
        module_structure,
        hotspots,
        reading_order,
    })
}

// ─── lightweight mode (remote Tier 1) ────────────────────────────────────────

/// Build overview data from cached metadata only (no source files downloaded).
fn build_lightweight_overview(repo: &Path, filter: Option<&str>) -> Result<OverviewOutput> {
    let project_name = project_name_from_path(repo);

    // 读取文件树
    let tree_path = repo.join("tree.json");
    let tree_json = fs::read_to_string(&tree_path)
        .with_context(|| format!("Failed to read tree cache: {}", tree_path.display()))?;
    let tree_items: Vec<CachedTreeItem> =
        serde_json::from_str(&tree_json).context("Failed to parse tree cache")?;

    // 统计语言（从文件扩展名推断）
    let languages = collect_language_stats_from_tree(&tree_items);
    let file_count = tree_items.len();

    // 读取依赖（从配置文件缓存）
    let dependencies = collect_dependencies_from_cache(repo)?;

    // 读取最近提交
    let recent_changes = collect_recent_changes_from_cache(repo)?;

    // 从文件树计算模块结构
    let module_structure = compute_module_structure_from_tree(&tree_items, filter);

    // 轻量级模式下估算文件重要性和阅读顺序（按文件大小）
    let file_ranks = compute_file_ranks_by_size(&tree_items, filter);
    let top_files = file_ranks
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, (path, size))| FileRankInfo {
            rank: i + 1,
            file: path.clone(),
            symbols: 0,
            pagerank: *size as f64,
            top_symbol: String::new(),
        })
        .collect();
    let reading_order = file_ranks
        .iter()
        .take(5)
        .map(|(path, _)| path.clone())
        .collect();

    Ok(OverviewOutput {
        project_name,
        symbol_count: 0,
        file_count,
        language_count: languages.len(),
        is_lightweight: true,
        languages,
        dependencies,
        recent_changes,
        top_files,
        entry_points: Vec::new(), // 轻量级模式不提供入口点
        module_structure,
        hotspots: Vec::new(), // 轻量级模式不提供热点
        reading_order,
    })
}

// ─── utility ────────────────────────────────────────────────────────────────

/// Extract a human-readable project name from a repo path.
/// For "." or relative paths, resolves via current directory.
fn project_name_from_path(repo: &Path) -> String {
    // 尝试规范化路径获取目录名
    let resolved = if repo == Path::new(".") {
        std::env::current_dir().ok()
    } else {
        repo.canonicalize().ok()
    };

    resolved
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            repo.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}

// ─── language stats ──────────────────────────────────────────────────────────

/// Count files per language from a full ScanResult.
fn collect_language_stats(scan_result: &crate::scan::ScanResult) -> Vec<LanguageStat> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for file in &scan_result.files {
        if let Some(ext) = file.path.extension().and_then(|e| e.to_str()) {
            let lang = extension_to_language(ext);
            *counts.entry(lang).or_insert(0) += 1;
        }
    }

    let mut stats: Vec<LanguageStat> = counts
        .into_iter()
        .map(|(language, file_count)| LanguageStat {
            language,
            file_count,
        })
        .collect();
    stats.sort_by_key(|b| std::cmp::Reverse(b.file_count));
    stats
}

/// Count files per language from cached tree items (path-based heuristic).
fn collect_language_stats_from_tree(items: &[CachedTreeItem]) -> Vec<LanguageStat> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for item in items {
        if item.item_type != "blob" {
            continue;
        }
        let p = Path::new(&item.path);
        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            let lang = extension_to_language(ext);
            *counts.entry(lang).or_insert(0) += 1;
        }
    }

    let mut stats: Vec<LanguageStat> = counts
        .into_iter()
        .map(|(language, file_count)| LanguageStat {
            language,
            file_count,
        })
        .collect();
    stats.sort_by_key(|b| std::cmp::Reverse(b.file_count));
    stats
}

/// Map a file extension to a human-readable language name.
fn extension_to_language(ext: &str) -> String {
    match ext {
        "rs" => "Rust".to_string(),
        "py" | "pyi" => "Python".to_string(),
        "js" => "JavaScript".to_string(),
        "ts" => "TypeScript".to_string(),
        "tsx" => "TypeScript (React)".to_string(),
        "jsx" => "JavaScript (React)".to_string(),
        "go" => "Go".to_string(),
        "java" => "Java".to_string(),
        "c" | "h" => "C".to_string(),
        "cpp" | "hpp" | "cc" | "cxx" => "C++".to_string(),
        "rb" => "Ruby".to_string(),
        "php" => "PHP".to_string(),
        "swift" => "Swift".to_string(),
        "kt" | "kts" => "Kotlin".to_string(),
        "scala" => "Scala".to_string(),
        "cs" => "C#".to_string(),
        "fs" | "fsx" => "F#".to_string(),
        "vue" => "Vue".to_string(),
        "svelte" => "Svelte".to_string(),
        "json" => "JSON".to_string(),
        "yaml" | "yml" => "YAML".to_string(),
        "toml" => "TOML".to_string(),
        "md" | "mdx" => "Markdown".to_string(),
        "css" => "CSS".to_string(),
        "scss" => "SCSS".to_string(),
        "less" => "Less".to_string(),
        "html" | "htm" => "HTML".to_string(),
        "xml" => "XML".to_string(),
        "sql" => "SQL".to_string(),
        "graphql" | "gql" => "GraphQL".to_string(),
        "proto" => "Protobuf".to_string(),
        "prisma" => "Prisma".to_string(),
        "r" => "R".to_string(),
        "jl" => "Julia".to_string(),
        "ex" | "exs" => "Elixir".to_string(),
        "erl" | "hrl" => "Erlang".to_string(),
        "dart" => "Dart".to_string(),
        "sh" | "bash" | "zsh" => "Shell".to_string(),
        "dockerfile" => "Docker".to_string(),
        other => {
            let upper = other.to_uppercase();
            if upper == other {
                upper
            } else {
                let mut c = other.chars();
                match c.next() {
                    None => other.to_string(),
                    Some(first) => first.to_uppercase().to_string() + c.as_str(),
                }
            }
        }
    }
}

// ─── dependencies ────────────────────────────────────────────────────────────

/// Extract dependencies from local config files.
fn collect_dependencies(repo: &Path) -> Result<Vec<DependencyInfo>> {
    let mut deps = Vec::new();

    // Cargo.toml
    if let Ok(content) = fs::read_to_string(repo.join("Cargo.toml")) {
        deps.extend(parse_cargo_dependencies(&content));
    }

    // package.json
    if let Ok(content) = fs::read_to_string(repo.join("package.json")) {
        deps.extend(parse_package_dependencies(&content));
    }

    // go.mod
    if let Ok(content) = fs::read_to_string(repo.join("go.mod")) {
        deps.extend(parse_gomod_dependencies(&content));
    }

    // pyproject.toml
    if let Ok(content) = fs::read_to_string(repo.join("pyproject.toml")) {
        deps.extend(parse_pyproject_dependencies(&content));
    }

    Ok(deps)
}

/// Extract dependencies from cached config files (remote lightweight mode).
fn collect_dependencies_from_cache(repo: &Path) -> Result<Vec<DependencyInfo>> {
    let mut deps = Vec::new();

    for filename in ["Cargo.toml", "package.json", "go.mod", "pyproject.toml"] {
        let path = repo.join(filename);
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path)
        {
            match filename {
                "Cargo.toml" => deps.extend(parse_cargo_dependencies(&content)),
                "package.json" => deps.extend(parse_package_dependencies(&content)),
                "go.mod" => deps.extend(parse_gomod_dependencies(&content)),
                "pyproject.toml" => deps.extend(parse_pyproject_dependencies(&content)),
                _ => {}
            }
        }
    }

    Ok(deps)
}

/// Parse `[dependencies]` section from Cargo.toml content.
fn parse_cargo_dependencies(content: &str) -> Vec<DependencyInfo> {
    let mut deps = Vec::new();
    let mut in_deps = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "[dependencies]" {
            in_deps = true;
            continue;
        }
        if in_deps && (trimmed.starts_with('[') || trimmed.is_empty()) {
            // 进入下一个 section 或空行，停止解析
            if trimmed.starts_with('[') {
                in_deps = false;
            }
            continue;
        }
        if in_deps
            && !trimmed.starts_with('#')
            && !trimmed.is_empty()
            && let Some((name, version)) = parse_cargo_dep_line(trimmed)
        {
            deps.push(DependencyInfo { name, version });
        }
    }

    deps
}

/// Parse a single Cargo dependency line like `clap = { version = "4", features = ["derive"] }`.
fn parse_cargo_dep_line(line: &str) -> Option<(String, String)> {
    let (name, rest) = match line.split_once('=') {
        Some(parts) => (parts.0.trim().to_string(), parts.1.trim()),
        None => return None,
    };

    // 简单字符串版本: `name = "1.0"`
    if rest.starts_with('"') {
        let version = rest.trim_matches('"').to_string();
        return Some((name, version));
    }

    // 内联表: `name = { version = "1.0", ... }`
    if rest.starts_with('{') && rest.ends_with('}') {
        let inner = &rest[1..rest.len() - 1];
        for part in inner.split(',') {
            let kv: Vec<&str> = part.trim().splitn(2, '=').collect();
            if kv.len() == 2 && kv[0].trim() == "version" {
                let version = kv[1].trim().trim_matches('"').to_string();
                return Some((name, version));
            }
        }
        // 没有 version 字段（如 git 依赖），返回占位
        return Some((name, "git".to_string()));
    }

    None
}

/// Parse dependencies from package.json content.
fn parse_package_dependencies(content: &str) -> Vec<DependencyInfo> {
    let mut deps = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        for field in ["dependencies", "devDependencies"] {
            if let Some(obj) = json.get(field).and_then(|v| v.as_object()) {
                for (name, version) in obj {
                    let ver_str = version.as_str().unwrap_or("?").to_string();
                    deps.push(DependencyInfo {
                        name: name.clone(),
                        version: ver_str,
                    });
                }
            }
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    deps.dedup_by(|a, b| a.name == b.name);
    deps
}

/// Parse dependencies from go.mod content.
fn parse_gomod_dependencies(content: &str) -> Vec<DependencyInfo> {
    let mut deps = Vec::new();
    let mut in_require = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("require (") {
            in_require = true;
            continue;
        }
        if in_require && trimmed == ")" {
            in_require = false;
            continue;
        }
        if in_require && !trimmed.is_empty() && !trimmed.starts_with("//") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                deps.push(DependencyInfo {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                });
            }
        }
        // 单行 require
        if !in_require && trimmed.starts_with("require ") && !trimmed.starts_with("require (") {
            let inner = trimmed.strip_prefix("require ").unwrap_or("");
            let parts: Vec<&str> = inner.split_whitespace().collect();
            if parts.len() >= 2 {
                deps.push(DependencyInfo {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                });
            }
        }
    }

    deps
}

/// Parse dependencies from pyproject.toml content (simple line-based, no TOML crate needed).
fn parse_pyproject_dependencies(content: &str) -> Vec<DependencyInfo> {
    let mut deps = Vec::new();
    let mut in_deps_list = false;
    let mut in_poetry_deps = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // [project] dependencies = [...]
        if trimmed.starts_with("dependencies = [") {
            in_deps_list = true;
            // 单行数组: dependencies = ["a", "b"]
            if trimmed.ends_with(']') && !trimmed.ends_with("[]") {
                let inner = &trimmed[trimmed.find('[').unwrap_or(0) + 1..trimmed.len() - 1];
                for part in inner.split(',') {
                    let dep_str = part.trim().trim_matches('"').trim_matches('\'');
                    if !dep_str.is_empty() {
                        deps.push(DependencyInfo {
                            name: dep_str.to_string(),
                            version: String::new(),
                        });
                    }
                }
                in_deps_list = false;
            }
            continue;
        }

        if in_deps_list {
            if trimmed == "]" {
                in_deps_list = false;
                continue;
            }
            let dep_str = trimmed
                .trim_matches(',')
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            if !dep_str.is_empty() {
                deps.push(DependencyInfo {
                    name: dep_str.to_string(),
                    version: String::new(),
                });
            }
            continue;
        }

        // [tool.poetry.dependencies]
        if trimmed == "[tool.poetry.dependencies]" {
            in_poetry_deps = true;
            continue;
        }
        if in_poetry_deps {
            if trimmed.starts_with('[') {
                in_poetry_deps = false;
                continue;
            }
            if let Some((name, version)) = parse_toml_kv_line(trimmed) {
                deps.push(DependencyInfo { name, version });
            }
        }
    }

    deps
}

/// Parse a TOML key-value line like `name = "value"` or `name = { version = "1.0" }`.
fn parse_toml_kv_line(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim().to_string();
    let value = value.trim();

    if value.starts_with('"') || value.starts_with('\'') {
        let v = value.trim_matches('"').trim_matches('\'').to_string();
        return Some((key, v));
    }

    if value.starts_with('{') {
        // Extract version from inline table
        for part in value.trim_matches(|c| c == '{' || c == '}').split(',') {
            let kv: Vec<&str> = part.trim().splitn(2, '=').collect();
            if kv.len() == 2 && kv[0].trim() == "version" {
                let v = kv[1]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                return Some((key, v));
            }
        }
    }

    Some((key, String::new()))
}

// ─── recent changes ──────────────────────────────────────────────────────────

/// Run `git log --oneline -10` to get recent changes.
fn collect_recent_changes(repo: &Path) -> Result<Vec<ChangeEntry>> {
    let output = Command::new("git")
        .args(["-C", &repo.to_string_lossy(), "log", "--oneline", "-10"])
        .output()
        .context("Failed to run git log. Is git installed and is this a git repository?")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries: Vec<ChangeEntry> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                Some(ChangeEntry {
                    sha: parts[0].to_string(),
                    message: parts[1].to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(entries)
}

/// Read recent commits from cached commits.json (remote lightweight mode).
fn collect_recent_changes_from_cache(repo: &Path) -> Result<Vec<ChangeEntry>> {
    let commits_path = repo.join("commits.json");
    if !commits_path.exists() {
        return Ok(Vec::new());
    }

    let json_str = fs::read_to_string(&commits_path)
        .with_context(|| format!("Failed to read commits cache: {}", commits_path.display()))?;

    let commits: Vec<crate::remote::CommitInfo> =
        serde_json::from_str(&json_str).context("Failed to parse commits cache")?;

    Ok(commits
        .into_iter()
        .map(|c| ChangeEntry {
            sha: c.sha,
            message: c.message,
        })
        .collect())
}

// ─── file PageRank (full mode) ───────────────────────────────────────────────

/// Aggregate per-symbol PageRank into per-file scores.
fn compute_file_pageranks(graph: &crate::graph::SymbolGraph) -> Vec<(PathBuf, f64)> {
    let mut file_scores: HashMap<PathBuf, f64> = HashMap::new();
    let mut file_symbol_counts: HashMap<PathBuf, usize> = HashMap::new();

    for symbol in graph.symbols.values() {
        let key = symbol.file.clone();
        *file_scores.entry(key.clone()).or_insert(0.0) += symbol.pagerank;
        *file_symbol_counts.entry(key).or_insert(0) += 1;
    }

    let mut result: Vec<(PathBuf, f64)> = file_scores.into_iter().collect();
    // 排序：PageRank 降序
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result
}

/// Build the top-N files list from file PageRank scores.
fn build_top_files(
    file_pageranks: &[(PathBuf, f64)],
    graph: &crate::graph::SymbolGraph,
    limit: usize,
    filter: Option<&str>,
) -> Vec<FileRankInfo> {
    file_pageranks
        .iter()
        .filter(|(path, _)| {
            if let Some(f) = filter {
                path.to_string_lossy()
                    .to_lowercase()
                    .contains(&f.to_lowercase())
            } else {
                true
            }
        })
        .take(limit)
        .enumerate()
        .map(|(i, (path, pr))| {
            let file_str = path.to_string_lossy().to_string();
            // 获取该文件中 PageRank 最高的符号
            let top_sym = graph
                .file_symbols
                .get(path)
                .and_then(|ids| {
                    ids.iter()
                        .filter_map(|id| graph.symbols.get(id))
                        .max_by(|a, b| {
                            a.pagerank
                                .partial_cmp(&b.pagerank)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                })
                .map(|s| s.name.clone())
                .unwrap_or_default();

            // 统计文件中的符号数量
            let sym_count = graph
                .file_symbols
                .get(path)
                .map(|ids| ids.len())
                .unwrap_or(0);

            FileRankInfo {
                rank: i + 1,
                file: file_str,
                symbols: sym_count,
                pagerank: *pr,
                top_symbol: top_sym,
            }
        })
        .collect()
}

// ─── entry points (full mode only) ───────────────────────────────────────────

/// Find entry points: function/main symbols with high PageRank.
fn find_entry_points(
    graph: &crate::graph::SymbolGraph,
    filter: Option<&str>,
) -> Vec<EntryPointInfo> {
    use crate::scan::parser::SymbolKind;

    let mut entries: Vec<EntryPointInfo> = graph
        .symbols
        .values()
        .filter(|sym| {
            // 入口点特征：main 函数 或 高 PageRank 的公开函数
            let is_main = sym.name == "main" && sym.kind == SymbolKind::Function;
            let is_significant = sym.pagerank > 0.01
                && matches!(
                    sym.kind,
                    SymbolKind::Function | SymbolKind::Method | SymbolKind::Struct
                );
            let passes_filter = match filter {
                Some(f) => sym
                    .file
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&f.to_lowercase()),
                None => true,
            };
            passes_filter && (is_main || is_significant)
        })
        .map(|sym| {
            // 判断入口类型
            let kind = if sym.name == "main" && sym.kind == SymbolKind::Function {
                "cli".to_string()
            } else if matches!(
                sym.kind,
                SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Trait
            ) {
                "type".to_string()
            } else {
                "function".to_string()
            };

            EntryPointInfo {
                kind,
                name: sym.name.clone(),
                file: sym.file.to_string_lossy().to_string(),
                line: sym.line,
                pagerank: sym.pagerank,
            }
        })
        .collect();

    // 按 PageRank 降序排列，取前 20 个
    entries.sort_by(|a, b| {
        b.pagerank
            .partial_cmp(&a.pagerank)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    entries.truncate(20);
    entries
}

// ─── module structure ────────────────────────────────────────────────────────

/// Build a 2-level directory tree from scan results.
fn compute_module_structure_from_scan(
    scan_result: &crate::scan::ScanResult,
    filter: Option<&str>,
) -> Vec<DirInfo> {
    let mut dir_map: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();

    for file in &scan_result.files {
        let path_str = file.path.to_string_lossy().to_string();
        if let Some(f) = filter
            && !path_str.to_lowercase().contains(&f.to_lowercase())
        {
            continue;
        }
        let p = Path::new(&path_str);
        let top_dir = p
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        let sub_dir = p.parent().and_then(|parent| {
            let parent_str = parent.to_string_lossy();
            if parent_str == top_dir || parent_str.is_empty() || parent_str == "." {
                None
            } else {
                Some(parent_str.to_string())
            }
        });

        let entry = dir_map.entry(top_dir).or_default();
        let key = sub_dir.unwrap_or_else(|| "(root)".to_string());
        *entry.entry(key).or_insert(0) += 1;
    }

    dir_map
        .into_iter()
        .map(|(top_dir, subs)| {
            let children: Vec<DirInfo> = subs
                .into_iter()
                .map(|(sub_name, count)| DirInfo {
                    path: sub_name,
                    file_count: count,
                    children: Vec::new(),
                })
                .collect();
            let total_files: usize = children.iter().map(|c| c.file_count).sum();
            DirInfo {
                path: top_dir,
                file_count: total_files,
                children,
            }
        })
        .collect()
}

/// Build a 2-level directory tree from cached tree items.
fn compute_module_structure_from_tree(
    items: &[CachedTreeItem],
    filter: Option<&str>,
) -> Vec<DirInfo> {
    let mut dir_map: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();

    for item in items {
        if item.item_type != "blob" {
            continue;
        }
        if let Some(f) = filter
            && !item.path.to_lowercase().contains(&f.to_lowercase())
        {
            continue;
        }

        let p = Path::new(&item.path);
        let top_dir = p
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        let sub_dir = p.parent().and_then(|parent| {
            let parent_str = parent.to_string_lossy();
            if parent_str == top_dir || parent_str.is_empty() || parent_str == "." {
                None
            } else {
                Some(parent_str.to_string())
            }
        });

        let entry = dir_map.entry(top_dir).or_default();
        let key = sub_dir.unwrap_or_else(|| "(root)".to_string());
        *entry.entry(key).or_insert(0) += 1;
    }

    dir_map
        .into_iter()
        .map(|(top_dir, subs)| {
            let children: Vec<DirInfo> = subs
                .into_iter()
                .map(|(sub_name, count)| DirInfo {
                    path: sub_name,
                    file_count: count,
                    children: Vec::new(),
                })
                .collect();
            let total_files: usize = children.iter().map(|c| c.file_count).sum();
            DirInfo {
                path: top_dir,
                file_count: total_files,
                children,
            }
        })
        .collect()
}

// ─── hotspots (full mode only) ──────────────────────────────────────────────

/// Compute file-level complexity hotspots: symbol density × PageRank.
fn compute_file_hotspots(
    graph: &crate::graph::SymbolGraph,
    file_pageranks: &[(PathBuf, f64)],
    filter: Option<&str>,
) -> Vec<HotspotInfo> {
    let pr_map: HashMap<&Path, f64> = file_pageranks
        .iter()
        .map(|(p, pr)| (p.as_path(), *pr))
        .collect();

    let mut scores: Vec<(&Path, f64, usize)> = graph
        .file_symbols
        .iter()
        .filter(|(path, _)| {
            if let Some(f) = filter {
                path.to_string_lossy()
                    .to_lowercase()
                    .contains(&f.to_lowercase())
            } else {
                true
            }
        })
        .map(|(path, sym_ids)| {
            let sym_count = sym_ids.len();
            let pr = pr_map.get(path.as_path()).copied().unwrap_or(0.0);
            let score = (sym_count as f64) * pr;
            (path.as_path(), score, sym_count)
        })
        .collect();

    // 按 score 降序排列
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scores
        .into_iter()
        .take(10)
        .enumerate()
        .map(|(i, (path, score, sym_count))| HotspotInfo {
            rank: i + 1,
            file: path.to_string_lossy().to_string(),
            score,
            symbols: sym_count,
            pagerank: pr_map.get(path).copied().unwrap_or(0.0),
        })
        .collect()
}

// ─── reading order ──────────────────────────────────────────────────────────

/// Suggest reading order: top 5 files by PageRank (full mode) or size (lightweight).
fn suggest_reading_order(file_pageranks: &[(PathBuf, f64)], filter: Option<&str>) -> Vec<String> {
    file_pageranks
        .iter()
        .filter(|(path, _)| {
            if let Some(f) = filter {
                path.to_string_lossy()
                    .to_lowercase()
                    .contains(&f.to_lowercase())
            } else {
                true
            }
        })
        .take(5)
        .map(|(path, _)| path.to_string_lossy().to_string())
        .collect()
}

/// Rank files by size (lightweight mode fallback).
fn compute_file_ranks_by_size(
    items: &[CachedTreeItem],
    filter: Option<&str>,
) -> Vec<(String, u64)> {
    let mut files: Vec<(&str, u64)> = items
        .iter()
        .filter(|item| {
            item.item_type == "blob"
                && item.size.is_some()
                && match filter {
                    Some(f) => item.path.to_lowercase().contains(&f.to_lowercase()),
                    None => true,
                }
        })
        .map(|item| (item.path.as_str(), item.size.unwrap_or(0)))
        .collect();

    files.sort_by_key(|b| std::cmp::Reverse(b.1));
    files
        .into_iter()
        .map(|(path, size)| (path.to_string(), size))
        .collect()
}

// ─── output writers ──────────────────────────────────────────────────────────

/// Write overview as JSON.
fn write_json(writer: &OutputWriter, data: &OverviewOutput) -> Result<()> {
    let json =
        serde_json::to_string_pretty(data).context("Failed to serialize overview to JSON")?;
    fs::write(writer.output_file(), json).context("Failed to write overview JSON output")?;
    Ok(())
}

/// Write overview as Markdown.
fn write_markdown(writer: &OutputWriter, data: &OverviewOutput) -> Result<()> {
    let mut out = String::new();

    // ── Title ──────────────────────────────────────────────────
    let mode_label = if data.is_lightweight {
        " (lightweight — metadata only)"
    } else {
        ""
    };
    out.push_str(&format!(
        "# Project Overview: {}{}\n\n",
        data.project_name, mode_label
    ));

    // ── Summary stats ──────────────────────────────────────────
    if data.is_lightweight {
        out.push_str(&format!(
            "**{} files** across **{} languages** (tree analysis, no source download)\n\n",
            data.file_count, data.language_count
        ));
    } else {
        out.push_str(&format!(
            "**{} symbols** across **{} files** ({} languages)\n\n",
            data.symbol_count, data.file_count, data.language_count
        ));
    }

    // ── Language Support ───────────────────────────────────────
    if !data.languages.is_empty() {
        out.push_str("## Language Support\n\n");
        out.push_str("| Language | Files |\n");
        out.push_str("|----------|-------|\n");
        for lang in &data.languages {
            out.push_str(&format!("| {} | {} |\n", lang.language, lang.file_count));
        }
        out.push('\n');
    }

    // ── Key Dependencies ──────────────────────────────────────
    if !data.dependencies.is_empty() {
        out.push_str("## Key Dependencies\n\n");
        out.push_str("| Package | Version |\n");
        out.push_str("|---------|----------|\n");
        for dep in &data.dependencies {
            out.push_str(&format!("| {} | {} |\n", dep.name, dep.version));
        }
        out.push('\n');
    }

    // ── Recent Changes ────────────────────────────────────────
    if !data.recent_changes.is_empty() {
        out.push_str("## Recent Changes\n\n");
        for entry in &data.recent_changes {
            out.push_str(&format!("- `{}` {}\n", entry.sha, entry.message));
        }
        out.push('\n');
    }

    // ── Top Files by PageRank ─────────────────────────────────
    if !data.top_files.is_empty() {
        if data.is_lightweight {
            out.push_str("## Largest Files (size-based ranking)\n\n");
            out.push_str("| Rank | File | Size (bytes) |\n");
            out.push_str("|------|------|-------------|\n");
            for f in &data.top_files {
                let size_kb = f.pagerank as u64 / 1024;
                out.push_str(&format!("| {} | `{}` | {} KB |\n", f.rank, f.file, size_kb));
            }
        } else {
            out.push_str("## Top Files by PageRank\n\n");
            out.push_str("| Rank | File | Symbols | PageRank | Top Symbol |\n");
            out.push_str("|------|------|---------|----------|------------|\n");
            for f in &data.top_files {
                out.push_str(&format!(
                    "| {} | `{}` | {} | {:.4} | `{}` |\n",
                    f.rank, f.file, f.symbols, f.pagerank, f.top_symbol
                ));
            }
        }
        out.push('\n');
    }

    // ── Entry Points ──────────────────────────────────────────
    if !data.entry_points.is_empty() {
        out.push_str("## Entry Points\n\n");
        for ep in &data.entry_points {
            out.push_str(&format!(
                "- `{}` **{}** — `{}:{}` (PR {:.4})\n",
                ep.kind, ep.name, ep.file, ep.line, ep.pagerank
            ));
        }
        out.push('\n');
    }

    // ── Module Structure ──────────────────────────────────────
    if !data.module_structure.is_empty() {
        out.push_str("## Module Structure\n\n");
        for dir in &data.module_structure {
            out.push_str(&format!(
                "- **`{}`** — {} files\n",
                dir.path, dir.file_count
            ));
            for child in &dir.children {
                out.push_str(&format!(
                    "  - `{}` — {} files\n",
                    child.path, child.file_count
                ));
            }
        }
        out.push('\n');
    }

    // ── Complexity Hotspots ───────────────────────────────────
    if !data.hotspots.is_empty() {
        out.push_str("## Complexity Hotspots\n\n");
        out.push_str("Ranked by symbol density × PageRank score.\n\n");
        out.push_str("| Rank | File | Score | Symbols | PageRank |\n");
        out.push_str("|------|------|-------|---------|----------|\n");
        for h in &data.hotspots {
            out.push_str(&format!(
                "| {} | `{}` | {:.2} | {} | {:.4} |\n",
                h.rank, h.file, h.score, h.symbols, h.pagerank
            ));
        }
        out.push('\n');
    }

    // ── Suggested Reading Order ───────────────────────────────
    if !data.reading_order.is_empty() {
        out.push_str("## Suggested Reading Order\n\n");
        for (i, file) in data.reading_order.iter().enumerate() {
            out.push_str(&format!("{}. `{}`\n", i + 1, file));
        }
        out.push('\n');
    }

    // ── Footer ────────────────────────────────────────────────
    if data.is_lightweight {
        out.push_str("---\n*Lightweight mode: metadata from GitHub API only. ");
        out.push_str(
            "Use `--full` with other commands to download source files for deeper analysis.*\n",
        );
    }

    fs::write(writer.output_file(), out).context("Failed to write overview markdown output")?;
    Ok(())
}

// ─── lightweight tree item ───────────────────────────────────────────────────

/// Matches the structure in remote/mod.rs for cached tree items.
#[derive(serde::Deserialize)]
struct CachedTreeItem {
    path: String,
    #[serde(rename = "item_type")]
    #[allow(dead_code)]
    item_type: String,
    #[allow(dead_code)]
    size: Option<u64>,
}

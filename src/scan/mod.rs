//! L2: Tree-sitter language-aware parsing.
//!
//! 3 阶段管线: 解析 → 建边 → 评分。
//! Phase A 仅实现阶段 1（解析 + 符号提取）。

pub mod parser;
pub mod queries;
pub mod stack;

use anyhow::Result;
use ignore::WalkBuilder;
use log::debug;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;

use parser::{CompiledQueries, ParsedFile, compile_queries, detect_language};

/// 项目扫描结果。
#[derive(Debug)]
pub struct ScanResult {
    /// 所有成功解析的文件
    pub files: Vec<ParsedFile>,
    /// 符号总数（调试/日志用）
    #[allow(dead_code)]
    pub symbol_count: usize,
}

/// 扫描项目，对每个支持的源文件执行 tree-sitter 解析。
///
/// 优化策略:
/// - 按语言预编译 Query 对象（每语言编译一次，非每文件）
/// - 使用 rayon 并行解析文件（CPU 密集型工作完美并行）
/// - 复用 `ignore` crate 的 walker，尊重 `.gitignore`
pub fn scan_project(root: &Path) -> Result<ScanResult> {
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .build();

    // 阶段 1: 收集所有需要解析的文件路径和内容（串行 I/O）
    let mut file_entries: Vec<(
        std::path::PathBuf,
        std::path::PathBuf,
        Vec<u8>,
        parser::Language,
    )> = Vec::new();
    let mut skipped = 0usize;

    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }

        let abs_path = entry.path();

        let lang = match detect_language(abs_path) {
            Some(l) => l,
            None => continue,
        };

        // 跳过过大的文件 (> 500KB)，tree-sitter 解析会很慢
        if let Ok(meta) = std::fs::metadata(abs_path)
            && meta.len() > 512 * 1024
        {
            debug!(
                "Skipping large file: {} ({}KB)",
                abs_path.display(),
                meta.len() / 1024
            );
            skipped += 1;
            continue;
        }

        let source = match std::fs::read(abs_path) {
            Ok(s) => s,
            Err(e) => {
                debug!("Cannot read {}: {}", abs_path.display(), e);
                continue;
            }
        };

        let rel_path = abs_path
            .strip_prefix(root)
            .unwrap_or(abs_path)
            .to_path_buf();
        file_entries.push((abs_path.to_path_buf(), rel_path, source, lang));
    }

    debug!(
        "Scan: {} files to parse, {} skipped (size/lang)",
        file_entries.len(),
        skipped
    );

    // 阶段 2: 按语言预编译 Query 对象（每语言仅编译一次）
    let compiled_queries: HashMap<parser::Language, CompiledQueries> = [
        parser::Language::Rust,
        parser::Language::Python,
        parser::Language::TypeScript,
        parser::Language::Go,
    ]
    .iter()
    .filter_map(|&lang| compile_queries(lang).map(|q| (lang, q)))
    .collect();

    // 阶段 3: 并行解析（rayon 并行迭代，CPU 密集型工作）
    let parse_results: Vec<(ParsedFile, usize)> = file_entries
        .par_iter()
        .filter_map(|(abs_path, rel_path, source, lang)| {
            let queries = compiled_queries.get(lang)?;
            let mut parsed = parser::parse_file_with_queries(abs_path, source, *lang, queries)?;
            // 将路径替换为相对于 root 的路径
            parsed.path = rel_path.clone();
            let sym_count = parsed.symbols.len();
            Some((parsed, sym_count))
        })
        .collect();

    let symbol_count: usize = parse_results.iter().map(|(_, c)| c).sum();
    let files: Vec<ParsedFile> = parse_results.into_iter().map(|(f, _)| f).collect();

    debug!(
        "Scan complete: {} files, {} symbols, {} skipped",
        files.len(),
        symbol_count,
        skipped
    );

    Ok(ScanResult {
        files,
        symbol_count,
    })
}

/// 在扫描结果中按名称搜索符号（不区分大小写）。
///
/// 返回匹配的 `(文件, 符号)` 对。
pub fn find_symbols<'a>(
    result: &'a ScanResult,
    query: &str,
) -> Vec<(&'a Path, &'a parser::ExtractedSymbol)> {
    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = query_lower.split_whitespace().collect();

    let mut matches = Vec::new();

    for file in &result.files {
        for sym in &file.symbols {
            let name_lower = sym.name.to_lowercase();
            // 名称包含任意查询词即匹配
            if terms.iter().any(|t| name_lower.contains(t)) {
                matches.push((file.path.as_path(), sym));
            }
        }
    }

    // 按匹配质量排序: 完全匹配 > 前缀匹配 > 包含匹配
    matches.sort_by(|a, b| {
        let a_exact = terms.iter().any(|t| a.1.name.to_lowercase() == *t);
        let b_exact = terms.iter().any(|t| b.1.name.to_lowercase() == *t);
        b_exact.cmp(&a_exact)
    });

    matches
}

/// 在扫描结果中按名称搜索调用引用（不区分大小写）。
pub fn find_call_refs<'a>(
    result: &'a ScanResult,
    query: &str,
) -> Vec<(&'a Path, &'a parser::CallRef)> {
    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = query_lower.split_whitespace().collect();

    let mut matches = Vec::new();

    for file in &result.files {
        for call in &file.calls {
            let name_lower = call.name.to_lowercase();
            if terms.iter().any(|t| name_lower.contains(t)) {
                matches.push((file.path.as_path(), call));
            }
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_project_finds_symbols() {
        let result = scan_project(Path::new(".")).expect("scan should succeed");
        assert!(
            result.symbol_count > 0,
            "should find symbols in this project"
        );
        assert!(result.files.len() > 0, "should find source files");

        // 验证至少能找到 Args 符号
        let has_args = result
            .files
            .iter()
            .any(|f| f.symbols.iter().any(|s| s.name == "Args"));
        assert!(has_args, "should find Args struct in src/cli.rs");
    }

    #[test]
    fn test_find_symbols_by_name() {
        let result = scan_project(Path::new(".")).expect("scan should succeed");
        let matches = find_symbols(&result, "Args");
        assert!(!matches.is_empty(), "should find Args symbols");

        // 应该找到 struct Args 的定义
        let args_struct = matches
            .iter()
            .find(|(_, s)| s.name == "Args" && s.kind == parser::SymbolKind::Struct);
        assert!(args_struct.is_some(), "should find struct Args definition");
    }

    #[test]
    fn test_find_call_refs() {
        let result = scan_project(Path::new(".")).expect("scan should succeed");
        let refs = find_call_refs(&result, "parse");
        // 项目中一定有 parse 相关的调用
        assert!(!refs.is_empty(), "should find parse-related calls");
    }
}

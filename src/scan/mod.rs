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
use std::path::Path;

use parser::{ParsedFile, detect_language, parse_file};

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
/// 复用 `ignore` crate 的 walker，尊重 `.gitignore`。
/// 返回 `ScanResult`，包含所有提取到的符号、导入、调用。
pub fn scan_project(root: &Path) -> Result<ScanResult> {
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .build();

    let mut files = Vec::new();
    let mut symbol_count = 0usize;
    let mut skipped = 0usize;

    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }

        let abs_path = entry.path();

        // 只处理支持的语言文件
        if detect_language(abs_path).is_none() {
            continue;
        }

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

        let rel_path = abs_path.strip_prefix(root).unwrap_or(abs_path);

        match parse_file(abs_path, &source) {
            Some(mut parsed) => {
                // 将路径替换为相对于 root 的路径
                parsed.path = rel_path.to_path_buf();
                symbol_count += parsed.symbols.len();
                debug!(
                    "Parsed {}: {} symbols, {} imports, {} calls",
                    rel_path.display(),
                    parsed.symbols.len(),
                    parsed.imports.len(),
                    parsed.calls.len()
                );
                files.push(parsed);
            }
            None => {
                debug!("Parse failed or unsupported: {}", rel_path.display());
                skipped += 1;
            }
        }
    }

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

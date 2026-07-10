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

use parser::{CompiledQueries, ExtractedSymbol, ParsedFile, compile_queries, detect_language};

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

    // 阶段 1: 收集所有需要解析的文件路径（串行 I/O，仅遍历目录 + 语言检测 + 大小过滤）
    let mut file_paths: Vec<(std::path::PathBuf, std::path::PathBuf, parser::Language)> =
        Vec::new();
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

        let rel_path = abs_path
            .strip_prefix(root)
            .unwrap_or(abs_path)
            .to_path_buf();
        file_paths.push((abs_path.to_path_buf(), rel_path, lang));
    }

    // 阶段 1.5: 并行读取文件内容（I/O 密集型，rayon 并行读取）
    let file_entries: Vec<(
        std::path::PathBuf,
        std::path::PathBuf,
        Vec<u8>,
        parser::Language,
    )> = file_paths
        .par_iter()
        .filter_map(|(abs_path, rel_path, lang)| match std::fs::read(abs_path) {
            Ok(source) => Some((abs_path.clone(), rel_path.clone(), source, *lang)),
            Err(e) => {
                debug!("Cannot read {}: {}", abs_path.display(), e);
                None
            }
        })
        .collect();

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

    // 按匹配质量排序: 完全匹配(exact=2) > 前缀匹配(prefix=1) > 包含匹配(contains=0)
    // 预计算匹配质量与稳定 tiebreaker，避免排序闭包内重复分配（REVIEW #59 + #67）
    let mut scored: Vec<(u8, &Path, &ExtractedSymbol)> = matches
        .into_iter()
        .map(|(path, sym)| {
            let name_lower = sym.name.to_lowercase();
            let quality = if terms.iter().any(|t| name_lower == *t) {
                2
            } else if terms.iter().any(|t| name_lower.starts_with(t)) {
                1
            } else {
                0
            };
            (quality, path, sym)
        })
        .collect();

    scored.sort_by(|a, b| {
        // 质量降序；同质量按 (file, line, name) 升序稳定排序
        b.0.cmp(&a.0)
            .then_with(|| a.1.cmp(b.1))
            .then_with(|| a.2.line.cmp(&b.2.line))
            .then_with(|| a.2.name.cmp(&b.2.name))
    });

    scored
        .into_iter()
        .map(|(_, path, sym)| (path, sym))
        .collect()
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

    // 按 (file, line) 稳定排序，保证 CLI 输出顺序可复现（REVIEW #58）
    matches.sort_by(|a, b| {
        a.0.display()
            .to_string()
            .cmp(&b.0.display().to_string())
            .then(a.1.line.cmp(&b.1.line))
    });

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
        assert!(!result.files.is_empty(), "should find source files");

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

    // ─── REVIEW #58: find_call_refs 输出顺序可复现 ───────────────────────────

    #[test]
    fn test_find_call_refs_order_deterministic() {
        use parser::CallRef;
        use std::path::PathBuf;

        // 构造确定性 ScanResult，模拟并行收集的非确定顺序
        let mut file_a = ParsedFile {
            path: PathBuf::from("a.rs"),
            symbols: Vec::new(),
            imports: Vec::new(),
            calls: vec![
                CallRef {
                    name: "parse".into(),
                    line: 20,
                },
                CallRef {
                    name: "parse".into(),
                    line: 5,
                },
            ],
        };
        let file_b = ParsedFile {
            path: PathBuf::from("b.rs"),
            symbols: Vec::new(),
            imports: Vec::new(),
            calls: vec![CallRef {
                name: "parse".into(),
                line: 10,
            }],
        };
        // 故意打乱插入顺序（交换两元素 line 值验证排序稳定）
        let tmp = file_a.calls[0].line;
        file_a.calls[0].line = file_a.calls[1].line;
        file_a.calls[1].line = tmp;
        let result = ScanResult {
            files: vec![file_b, file_a], // b 在 a 前，验证按 (file,line) 排序
            symbol_count: 0,
        };

        let refs = find_call_refs(&result, "parse");
        assert_eq!(refs.len(), 3);
        // 期望顺序: a.rs:5, a.rs:20, b.rs:10
        assert_eq!(refs[0].0.to_string_lossy(), "a.rs");
        assert_eq!(refs[0].1.line, 5);
        assert_eq!(refs[1].0.to_string_lossy(), "a.rs");
        assert_eq!(refs[1].1.line, 20);
        assert_eq!(refs[2].0.to_string_lossy(), "b.rs");
        assert_eq!(refs[2].1.line, 10);
    }

    // ─── REVIEW #59: find_symbols 三级匹配质量排序 ──────────────────────────

    #[test]
    fn test_find_symbols_match_quality_ordering() {
        use parser::{ExtractedSymbol, SymbolKind};
        use std::path::PathBuf;

        let sym_exact = ExtractedSymbol {
            name: "middleware".into(),
            kind: SymbolKind::Function,
            line: 1,
            end_line: 1,
            signature: String::new(),
        };
        let sym_prefix = ExtractedSymbol {
            name: "middleware_auth".into(),
            kind: SymbolKind::Function,
            line: 2,
            end_line: 2,
            signature: String::new(),
        };
        let sym_contains = ExtractedSymbol {
            name: "http_middleware_cfg".into(),
            kind: SymbolKind::Function,
            line: 3,
            end_line: 3,
            signature: String::new(),
        };

        let file = ParsedFile {
            path: PathBuf::from("m.rs"),
            symbols: vec![sym_contains.clone(), sym_prefix.clone(), sym_exact.clone()],
            imports: Vec::new(),
            calls: Vec::new(),
        };
        // 打乱顺序验证稳定排序
        let result = ScanResult {
            files: vec![file],
            symbol_count: 3,
        };

        let matches = find_symbols(&result, "middleware");
        assert_eq!(matches.len(), 3);
        // 期望: exact(middleware) > prefix(middleware_*) > contains(*_middleware_*)
        assert_eq!(matches[0].1.name, "middleware");
        assert_eq!(matches[1].1.name, "middleware_auth");
        assert_eq!(matches[2].1.name, "http_middleware_cfg");

        // 两次运行顺序一致（可复现）
        let again = find_symbols(&result, "middleware");
        let names: Vec<&str> = matches.iter().map(|(_, s)| s.name.as_str()).collect();
        let again_names: Vec<&str> = again.iter().map(|(_, s)| s.name.as_str()).collect();
        assert_eq!(names, again_names);
    }
}

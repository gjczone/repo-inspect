//! 3 阶段建图: ScanResult → SymbolGraph。
//!
//! Phase 1: 提取所有符号到图中
//! Phase 2: 建边（call/ref/import）
//! Phase 3: 由外部调用 PageRank（不在此模块）

use super::{Edge, EdgeKind, Symbol, SymbolGraph, make_symbol_id};
use crate::scan::ScanResult;
use crate::scan::parser::Language;
use log::debug;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 从扫描结果构建符号依赖图。
///
/// Phase 1: 所有符号 → 图节点
/// Phase 2: call 边 + ref 边（同文件内按名称匹配）
pub fn build_graph(scan_result: &ScanResult) -> SymbolGraph {
    let mut graph = SymbolGraph::new();

    // Phase 1: 添加所有符号
    for file in &scan_result.files {
        for sym in &file.symbols {
            let id = make_symbol_id(&file.path, &sym.name, sym.line);
            graph.add_symbol(Symbol {
                id,
                name: sym.name.clone(),
                kind: sym.kind,
                file: file.path.clone(),
                line: sym.line,
                end_line: sym.end_line,
                signature: sym.signature.clone(),
                pagerank: 0.0,
            });
        }
    }

    debug!(
        "Graph Phase 1: {} symbols from {} files",
        graph.symbol_count(),
        scan_result.files.len()
    );

    // Phase 2: 建边
    // 遍历每个文件的 call refs，匹配 callee 符号
    for file in &scan_result.files {
        let file_sym_ids = graph
            .file_symbols
            .get(&file.path)
            .cloned()
            .unwrap_or_default();

        for call in &file.calls {
            // 查找 callee: 全局 name_index 匹配
            let callee_ids: Vec<_> = graph
                .name_index
                .get(&call.name)
                .cloned()
                .unwrap_or_default();

            if callee_ids.is_empty() {
                continue;
            }

            // 查找 caller: 包含此 call 行号的最内层符号
            let caller_id = find_caller_symbol(&file_sym_ids, &graph, call.line);
            let caller_id = match caller_id {
                Some(id) => id,
                None => continue,
            };

            for callee_id in &callee_ids {
                if *callee_id == caller_id {
                    continue; // 不自环
                }

                // 判断是 Call 还是 Ref
                let (kind, weight) = if is_same_file(&caller_id, callee_id) {
                    (EdgeKind::Ref, 0.5)
                } else {
                    (EdgeKind::Call, 1.0)
                };

                graph.add_edge(Edge {
                    source: caller_id.clone(),
                    target: callee_id.clone(),
                    kind,
                    weight,
                });
            }
        }
    }

    // Phase 2b: import 边
    // 收集所有已知文件路径，用于 import 解析
    let known_files: HashSet<&Path> = graph.file_symbols.keys().map(|p| p.as_path()).collect();
    let known_file_strings: HashSet<String> = known_files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    for file in &scan_result.files {
        let lang = crate::scan::parser::detect_language(&PathBuf::from(&file.path));
        if lang.is_none() {
            continue;
        }

        let file_sym_ids = graph
            .file_symbols
            .get(&file.path)
            .cloned()
            .unwrap_or_default();

        for import in &file.imports {
            let resolved = resolve_import(
                &import.module,
                &file.path,
                lang.unwrap(),
                &known_file_strings,
            );
            let resolved = match resolved {
                Some(r) => r,
                None => continue,
            };

            // 查找目标文件的符号
            let target_sym_ids = graph
                .file_symbols
                .get(&resolved)
                .cloned()
                .unwrap_or_default();

            if target_sym_ids.is_empty() {
                continue;
            }

            // 当前文件所有符号 → 目标文件所有符号 (weight=0.3)
            for src_id in &file_sym_ids {
                for tgt_id in &target_sym_ids {
                    if src_id != tgt_id {
                        graph.add_edge(Edge {
                            source: src_id.clone(),
                            target: tgt_id.clone(),
                            kind: EdgeKind::Import,
                            weight: 0.3,
                        });
                    }
                }
            }
        }
    }

    debug!("Graph Phase 2: {} edges", graph.edge_count());

    graph
}

/// 找到包含指定行号的最内层（最窄范围）符号。
fn find_caller_symbol(
    file_sym_ids: &[super::SymbolId],
    graph: &SymbolGraph,
    call_line: usize,
) -> Option<super::SymbolId> {
    let mut best: Option<(&super::SymbolId, usize)> = None;

    for id in file_sym_ids {
        if let Some(sym) = graph.symbols.get(id)
            && sym.line <= call_line
            && call_line <= sym.end_line
        {
            let range = sym.end_line - sym.line;
            match best {
                None => best = Some((id, range)),
                Some((_, best_range)) if range < best_range => best = Some((id, range)),
                _ => {}
            }
        }
    }

    best.map(|(id, _)| id.clone())
}

/// 判断两个 SymbolId 是否在同一文件。
fn is_same_file(a: &str, b: &str) -> bool {
    let file_a = a.split("::").next().unwrap_or("");
    let file_b = b.split("::").next().unwrap_or("");
    file_a == file_b
}

/// 解析 import 路径到实际文件路径。
///
/// 按语言规则解析，返回相对于 repo root 的路径。
/// `known_files` 是项目中已知的文件路径集合，用于验证候选路径。
fn resolve_import(
    module: &str,
    from_file: &Path,
    lang: Language,
    known_files: &HashSet<String>,
) -> Option<PathBuf> {
    let from_dir = from_file.parent().unwrap_or(Path::new("."));

    match lang {
        Language::Rust => resolve_rust_import(module, from_dir, known_files),
        Language::Python => resolve_python_import(module, from_dir, known_files),
        Language::TypeScript => resolve_ts_import(module, from_dir, known_files),
        Language::Go => resolve_go_import(module, known_files),
    }
}

/// Rust import 解析:
/// - `crate::foo::bar` → `src/foo/bar.rs` 或 `src/foo/bar/mod.rs`
/// - `super::foo` → `../foo.rs` 相对于当前文件
/// - `foo` (单标识符) → `./foo.rs` 或 `./foo/mod.rs`
fn resolve_rust_import(
    module: &str,
    from_dir: &Path,
    known_files: &HashSet<String>,
) -> Option<PathBuf> {
    if module.starts_with("crate::") {
        // crate::scan::parser → src/scan/parser.rs 或 src/scan/parser/mod.rs
        let rel = module.strip_prefix("crate::").unwrap();
        let path_str = rel.replace("::", "/");
        let candidates = vec![
            format!("src/{}.rs", path_str),
            format!("src/{}/mod.rs", path_str),
            format!("{}.rs", path_str),
            format!("{}/mod.rs", path_str),
        ];
        return find_existing(&candidates, known_files);
    }

    if module.starts_with("super::") {
        // super::foo → ../foo.rs 相对于当前文件
        let rel = module.strip_prefix("super::").unwrap();
        let path_str = rel.replace("::", "/");
        let parent = from_dir.parent().unwrap_or(Path::new("."));
        let candidates = vec![
            format!("{}/{}.rs", parent.display(), path_str),
            format!("{}/{}/mod.rs", parent.display(), path_str),
        ];
        return find_existing(&candidates, known_files);
    }

    if !module.contains("::") {
        // 单标识符: 同级模块
        let candidates = vec![
            format!("{}/{}.rs", from_dir.display(), module),
            format!("{}/{}/mod.rs", from_dir.display(), module),
        ];
        return find_existing(&candidates, known_files);
    }

    // 多段路径 (如 std::collections) — 通常是外部 crate，跳过
    None
}

/// Python import 解析:
/// - `foo.bar.baz` → `foo/bar/baz.py` 或 `foo/bar/baz/__init__.py`
/// - `.foo` → 相对于当前文件
fn resolve_python_import(
    module: &str,
    from_dir: &Path,
    known_files: &HashSet<String>,
) -> Option<PathBuf> {
    if module.starts_with('.') {
        // 相对导入
        let rel = module.trim_start_matches('.');
        let path_str = rel.replace('.', "/");
        let candidates = vec![
            format!("{}/{}.py", from_dir.display(), path_str),
            format!("{}/{}/__init__.py", from_dir.display(), path_str),
        ];
        return find_existing(&candidates, known_files);
    }

    // 绝对导入: foo.bar.baz
    let path_str = module.replace('.', "/");
    let candidates = vec![
        format!("{}.py", path_str),
        format!("{}/__init__.py", path_str),
        format!("src/{}.py", path_str),
        format!("src/{}/__init__.py", path_str),
    ];
    find_existing(&candidates, known_files)
}

/// TypeScript/JavaScript import 解析:
/// - `./utils` → `./utils.ts`, `./utils/index.ts`, `./utils.js` 等
fn resolve_ts_import(
    module: &str,
    from_dir: &Path,
    known_files: &HashSet<String>,
) -> Option<PathBuf> {
    if module.starts_with('.') {
        let base = from_dir.join(module);
        let base_str = base.display().to_string();
        let candidates = vec![
            format!("{}.ts", base_str),
            format!("{}.tsx", base_str),
            format!("{}.js", base_str),
            format!("{}/index.ts", base_str),
            format!("{}/index.tsx", base_str),
            format!("{}/index.js", base_str),
        ];
        return find_existing(&candidates, known_files);
    }

    // 非相对导入 (bare specifier) — 外部包，跳过
    None
}

/// Go import 解析:
/// - 本地路径 `"./foo"` → `foo.go`
/// - 外部路径 → 跳过
fn resolve_go_import(module: &str, known_files: &HashSet<String>) -> Option<PathBuf> {
    let clean = module.trim_matches('"');
    if clean.starts_with('.') || clean.starts_with('/') {
        let candidates = vec![
            format!("{}.go", clean),
            format!("{}/{}.go", clean, clean.rsplit('/').next().unwrap_or("")),
        ];
        return find_existing(&candidates, known_files);
    }
    None
}

/// 在候选路径中找到第一个在 known_files 中存在的。
fn find_existing(candidates: &[String], known_files: &HashSet<String>) -> Option<PathBuf> {
    for c in candidates {
        if known_files.contains(c) {
            return Some(PathBuf::from(c));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::parser::{CallRef, ExtractedSymbol, ParsedFile, SymbolKind};
    use std::path::PathBuf;

    fn make_test_scan_result() -> ScanResult {
        // 模拟: main.rs 调用 run(), run 定义在 find_how.rs
        let main_file = ParsedFile {
            path: PathBuf::from("src/main.rs"),
            symbols: vec![ExtractedSymbol {
                name: "main".to_string(),
                kind: SymbolKind::Function,
                line: 10,
                end_line: 20,
                signature: "fn main()".to_string(),
            }],
            imports: vec![],
            calls: vec![
                CallRef {
                    name: "run".to_string(),
                    line: 15,
                },
                CallRef {
                    name: "Args".to_string(),
                    line: 12,
                },
            ],
        };

        let find_how_file = ParsedFile {
            path: PathBuf::from("src/commands/find_how.rs"),
            symbols: vec![ExtractedSymbol {
                name: "run".to_string(),
                kind: SymbolKind::Function,
                line: 7,
                end_line: 25,
                signature: "pub fn run(...)".to_string(),
            }],
            imports: vec![],
            calls: vec![],
        };

        let cli_file = ParsedFile {
            path: PathBuf::from("src/cli.rs"),
            symbols: vec![ExtractedSymbol {
                name: "Args".to_string(),
                kind: SymbolKind::Struct,
                line: 12,
                end_line: 27,
                signature: "pub struct Args {".to_string(),
            }],
            imports: vec![],
            calls: vec![],
        };

        ScanResult {
            files: vec![main_file, find_how_file, cli_file],
            symbol_count: 3,
        }
    }

    #[test]
    fn test_build_graph_from_scan() {
        let scan_result = make_test_scan_result();
        let graph = build_graph(&scan_result);

        assert_eq!(graph.symbol_count(), 3);
        assert!(
            graph.edge_count() > 0,
            "should have edges from main to run/Args"
        );
    }

    #[test]
    fn test_build_graph_call_edges() {
        let scan_result = make_test_scan_result();
        let graph = build_graph(&scan_result);

        // main::run() 应该产生 main → run 的 Call 边（跨文件）
        let main_id = make_symbol_id(std::path::Path::new("src/main.rs"), "main", 10);
        let run_id = make_symbol_id(std::path::Path::new("src/commands/find_how.rs"), "run", 7);

        let main_outgoing = graph.outgoing.get(&main_id);
        assert!(main_outgoing.is_some(), "main should have outgoing edges");

        let has_call_to_run = main_outgoing
            .unwrap()
            .iter()
            .any(|e| e.target == run_id && e.kind == EdgeKind::Call);
        assert!(has_call_to_run, "main should have Call edge to run");

        // run 应该有 incoming edge from main
        let run_incoming = graph.incoming.get(&run_id);
        assert!(run_incoming.is_some(), "run should have incoming edges");
    }

    #[test]
    fn test_resolve_rust_crate_path() {
        let known: HashSet<String> = ["src/scan/parser.rs", "src/scan/mod.rs"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let result = resolve_rust_import("crate::scan::parser", Path::new("src"), &known);
        assert_eq!(result, Some(PathBuf::from("src/scan/parser.rs")));

        // mod.rs fallback
        let known2: HashSet<String> = ["src/graph/mod.rs"].iter().map(|s| s.to_string()).collect();
        let result2 = resolve_rust_import("crate::graph", Path::new("src"), &known2);
        assert_eq!(result2, Some(PathBuf::from("src/graph/mod.rs")));
    }

    #[test]
    fn test_resolve_rust_mod_path() {
        let known: HashSet<String> = ["src/scan/queries.rs"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let result = resolve_rust_import("queries", Path::new("src/scan"), &known);
        assert_eq!(result, Some(PathBuf::from("src/scan/queries.rs")));
    }

    #[test]
    fn test_resolve_python_dotted() {
        let known: HashSet<String> = ["foo/bar.py", "foo/bar/__init__.py"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let result = resolve_python_import("foo.bar", Path::new("."), &known);
        assert_eq!(result, Some(PathBuf::from("foo/bar.py")));
    }

    #[test]
    fn test_import_edges_created() {
        let mut scan_result = make_test_scan_result();
        // main.rs 有 import: use crate::commands::find_how
        scan_result.files[0]
            .imports
            .push(crate::scan::parser::ImportDecl {
                module: "crate::commands::find_how".to_string(),
                line: 1,
            });

        let graph = build_graph(&scan_result);
        // 应该有 import 边从 main 的符号到 find_how 的符号
        let main_id = make_symbol_id(Path::new("src/main.rs"), "main", 10);
        let edges = graph.outgoing.get(&main_id);
        assert!(edges.is_some());
        let has_import = edges.unwrap().iter().any(|e| e.kind == EdgeKind::Import);
        assert!(
            has_import,
            "should have import edge from main to find_how symbols"
        );
    }
}

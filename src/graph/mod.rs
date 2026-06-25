//! L3: 符号依赖图引擎。
//!
//! 核心数据结构: SymbolGraph（符号图）、Symbol（符号节点）、Edge（有向边）。
//! 子模块: builder（建图）、pagerank（评分）、traverse（遍历）。

pub mod builder;
pub mod pagerank;
pub mod traverse;

use crate::scan::parser::SymbolKind;
use std::collections::HashMap;
use std::path::PathBuf;

/// 符号唯一标识 — `{file}::{name}::{line}`。
///
/// 包含文件路径和行号，确保同名符号不冲突。
pub type SymbolId = String;

/// 图中的符号节点。
#[derive(Debug, Clone)]
pub struct Symbol {
    /// 唯一标识
    pub id: SymbolId,
    /// 符号名称
    pub name: String,
    /// 符号类别
    pub kind: SymbolKind,
    /// 文件路径（相对于 repo root）
    pub file: PathBuf,
    /// 起始行（1-based）
    pub line: usize,
    /// 结束行（1-based）
    pub end_line: usize,
    /// 签名预览
    pub signature: String,
    /// PageRank 评分（建图后由 pagerank 模块填充）
    pub pagerank: f64,
}

/// 有向边类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// 导入关系（weight=0.3）
    Import,
    /// 函数调用（weight=1.0）
    Call,
    /// 同文件引用（weight=0.5）
    Ref,
}

/// 有向边 — 从 source 符号指向 target 符号。
#[derive(Debug, Clone)]
pub struct Edge {
    /// 源符号 ID
    pub source: SymbolId,
    /// 目标符号 ID
    pub target: SymbolId,
    /// 边类型
    pub kind: EdgeKind,
    /// 权重
    pub weight: f64,
}

/// 符号依赖图。
///
/// 存储所有符号节点和有向边，支持高效的邻居查询和名称查找。
#[derive(Debug)]
pub struct SymbolGraph {
    /// 所有符号节点，按 SymbolId 索引
    pub symbols: HashMap<SymbolId, Symbol>,
    /// 出边: symbol_id → 向外的边列表
    pub outgoing: HashMap<SymbolId, Vec<Edge>>,
    /// 入边: symbol_id → 指向该符号的边列表
    pub incoming: HashMap<SymbolId, Vec<Edge>>,
    /// 名称索引: name → symbol_ids（O(1) 查找）
    pub name_index: HashMap<String, Vec<SymbolId>>,
    /// 文件到符号的映射: file → symbol_ids
    pub file_symbols: HashMap<PathBuf, Vec<SymbolId>>,
}

/// 生成符号唯一 ID。
///
/// 格式: `{file}::{name}::{line}`，确保同名符号在不同文件/行号不冲突。
pub fn make_symbol_id(file: &std::path::Path, name: &str, line: usize) -> SymbolId {
    format!("{}::{}::{}", file.display(), name, line)
}

impl SymbolGraph {
    /// 创建空图。
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            name_index: HashMap::new(),
            file_symbols: HashMap::new(),
        }
    }

    /// 添加符号节点。如果 ID 已存在则跳过。
    pub fn add_symbol(&mut self, symbol: Symbol) {
        let id = symbol.id.clone();
        let name = symbol.name.clone();
        let file = symbol.file.clone();

        // 更新 name_index
        self.name_index.entry(name).or_default().push(id.clone());

        // 更新 file_symbols
        self.file_symbols.entry(file).or_default().push(id.clone());

        self.symbols.insert(id, symbol);
    }

    /// 添加有向边。自动维护 outgoing 和 incoming 索引。
    /// 去重：相同 source+target+kind 的边不重复添加。
    pub fn add_edge(&mut self, edge: Edge) {
        // 去重检查
        if let Some(edges) = self.outgoing.get(&edge.source)
            && edges
                .iter()
                .any(|e| e.target == edge.target && e.kind == edge.kind)
        {
            return;
        }

        // 维护 outgoing
        self.outgoing
            .entry(edge.source.clone())
            .or_default()
            .push(edge.clone());

        // 维护 incoming
        self.incoming
            .entry(edge.target.clone())
            .or_default()
            .push(edge);
    }

    /// 图中的符号总数。
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// 图中的边总数。
    pub fn edge_count(&self) -> usize {
        self.outgoing.values().map(|v| v.len()).sum()
    }

    /// 按名称查找符号（使用 name_index，O(1) 查找）。
    pub fn find_by_name(&self, name: &str) -> Vec<&Symbol> {
        self.name_index
            .get(name)
            .map(|ids| ids.iter().filter_map(|id| self.symbols.get(id)).collect())
            .unwrap_or_default()
    }

    /// 按不区分大小写的名称查找符号。
    pub fn find_by_name_ignore_case(&self, query: &str) -> Vec<&Symbol> {
        let query_lower = query.to_lowercase();
        let terms: Vec<&str> = query_lower.split_whitespace().collect();

        self.symbols
            .values()
            .filter(|sym| {
                let name_lower = sym.name.to_lowercase();
                terms.iter().any(|t| name_lower.contains(t))
            })
            .collect()
    }

    /// 获取符号的 PageRank 分数，按降序排列。
    pub fn top_symbols(&self, limit: usize) -> Vec<&Symbol> {
        let mut syms: Vec<&Symbol> = self.symbols.values().collect();
        syms.sort_by(|a, b| {
            b.pagerank
                .partial_cmp(&a.pagerank)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        syms.into_iter().take(limit).collect()
    }
}

impl Default for SymbolGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_symbol(name: &str, file: &str, line: usize) -> Symbol {
        Symbol {
            id: make_symbol_id(std::path::Path::new(file), name, line),
            name: name.to_string(),
            kind: SymbolKind::Function,
            file: PathBuf::from(file),
            line,
            end_line: line,
            signature: format!("fn {}()", name),
            pagerank: 0.0,
        }
    }

    #[test]
    fn test_graph_add_symbol() {
        let mut graph = SymbolGraph::new();
        let sym = make_test_symbol("main", "src/main.rs", 10);
        graph.add_symbol(sym);

        assert_eq!(graph.symbol_count(), 1);
        assert_eq!(graph.find_by_name("main").len(), 1);
        assert_eq!(
            graph
                .file_symbols
                .get(PathBuf::from("src/main.rs").as_path())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn test_graph_add_symbol_duplicate_id_skipped() {
        let mut graph = SymbolGraph::new();
        graph.add_symbol(make_test_symbol("main", "src/main.rs", 10));
        graph.add_symbol(make_test_symbol("main", "src/main.rs", 10)); // 重复
        assert_eq!(graph.symbol_count(), 1);
    }

    #[test]
    fn test_graph_add_edge() {
        let mut graph = SymbolGraph::new();
        graph.add_symbol(make_test_symbol("main", "src/main.rs", 10));
        graph.add_symbol(make_test_symbol("run", "src/commands/find_how.rs", 7));

        let main_id = make_symbol_id(std::path::Path::new("src/main.rs"), "main", 10);
        let run_id = make_symbol_id(std::path::Path::new("src/commands/find_how.rs"), "run", 7);

        graph.add_edge(Edge {
            source: main_id.clone(),
            target: run_id.clone(),
            kind: EdgeKind::Call,
            weight: 1.0,
        });

        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.outgoing.get(&main_id).unwrap().len(), 1);
        assert_eq!(graph.incoming.get(&run_id).unwrap().len(), 1);
    }

    #[test]
    fn test_graph_add_edge_dedup() {
        let mut graph = SymbolGraph::new();
        graph.add_symbol(make_test_symbol("a", "a.rs", 1));
        graph.add_symbol(make_test_symbol("b", "b.rs", 1));

        let a_id = make_symbol_id(std::path::Path::new("a.rs"), "a", 1);
        let b_id = make_symbol_id(std::path::Path::new("b.rs"), "b", 1);

        graph.add_edge(Edge {
            source: a_id.clone(),
            target: b_id.clone(),
            kind: EdgeKind::Call,
            weight: 1.0,
        });
        graph.add_edge(Edge {
            source: a_id.clone(),
            target: b_id.clone(),
            kind: EdgeKind::Call,
            weight: 1.0,
        }); // 重复

        assert_eq!(graph.edge_count(), 1, "duplicate edges should be deduped");
    }

    #[test]
    fn test_graph_find_by_name_ignore_case() {
        let mut graph = SymbolGraph::new();
        graph.add_symbol(make_test_symbol("FindHowArgs", "src/cli.rs", 46));
        graph.add_symbol(make_test_symbol("Args", "src/cli.rs", 12));

        let results = graph.find_by_name_ignore_case("args");
        assert_eq!(results.len(), 2, "should find both Args and FindHowArgs");
    }

    #[test]
    fn test_make_symbol_id() {
        let id = make_symbol_id(std::path::Path::new("src/cli.rs"), "Args", 12);
        assert_eq!(id, "src/cli.rs::Args::12");
    }
}

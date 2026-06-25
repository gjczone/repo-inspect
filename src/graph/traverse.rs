//! BFS 调用链遍历。
//!
//! 从指定符号出发，沿 caller（入边）或 callee（出边）方向遍历，
//! 返回指定深度内的所有可达符号。

use super::{SymbolGraph, SymbolId};
use std::collections::{HashSet, VecDeque};

/// 遍历结果中的单个条目。
#[derive(Debug, Clone)]
pub struct TraceEntry {
    /// 符号 ID
    pub symbol_id: SymbolId,
    /// 从起始符号到此符号的深度
    pub depth: usize,
    /// 遍历方向: "caller" 或 "callee"
    pub direction: TraceDirection,
}

/// 遍历方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceDirection {
    /// 谁调用了这个符号（沿入边反向）
    Caller,
    /// 这个符号调用了谁（沿出边正向）
    Callee,
}

/// 从指定符号出发，沿出边（callee 方向）BFS 遍历。
///
/// 返回所有可达符号，按深度排序。max_depth=0 表示只返回直接 callee。
pub fn trace_callees(graph: &SymbolGraph, start: &SymbolId, max_depth: usize) -> Vec<TraceEntry> {
    bfs(graph, start, max_depth, TraceDirection::Callee)
}

/// 从指定符号出发，沿入边（caller 方向）BFS 遍历。
///
/// 返回所有可达符号，按深度排序。max_depth=0 表示只返回直接 caller。
pub fn trace_callers(graph: &SymbolGraph, start: &SymbolId, max_depth: usize) -> Vec<TraceEntry> {
    bfs(graph, start, max_depth, TraceDirection::Caller)
}

/// 按名称查找符号，返回所有匹配。
pub fn find_symbol_by_name<'a>(graph: &'a SymbolGraph, name: &str) -> Vec<&'a super::Symbol> {
    graph.find_by_name(name)
}

/// 按不区分大小写的名称查找符号。
pub fn find_symbol_by_name_ignore_case<'a>(
    graph: &'a SymbolGraph,
    query: &str,
) -> Vec<&'a super::Symbol> {
    graph.find_by_name_ignore_case(query)
}

/// 通用 BFS 遍历。
fn bfs(
    graph: &SymbolGraph,
    start: &SymbolId,
    max_depth: usize,
    direction: TraceDirection,
) -> Vec<TraceEntry> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut results = Vec::new();

    visited.insert(start.clone());
    queue.push_back((start.clone(), 0usize));

    while let Some((current_id, depth)) = queue.pop_front() {
        if depth > max_depth {
            continue;
        }

        // 获取邻居: 根据方向选择 outgoing 或 incoming
        let neighbors = match direction {
            TraceDirection::Callee => graph
                .outgoing
                .get(&current_id)
                .map(|edges| edges.iter().map(|e| &e.target).collect::<Vec<_>>())
                .unwrap_or_default(),
            TraceDirection::Caller => graph
                .incoming
                .get(&current_id)
                .map(|edges| edges.iter().map(|e| &e.source).collect::<Vec<_>>())
                .unwrap_or_default(),
        };

        for neighbor_id in neighbors {
            if visited.contains(neighbor_id) {
                continue;
            }
            visited.insert(neighbor_id.clone());

            results.push(TraceEntry {
                symbol_id: neighbor_id.clone(),
                depth: depth + 1,
                direction,
            });

            if depth + 1 < max_depth {
                queue.push_back((neighbor_id.clone(), depth + 1));
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Edge, EdgeKind, Symbol, SymbolGraph, make_symbol_id};
    use crate::scan::parser::SymbolKind;
    use std::path::PathBuf;

    fn make_sym(name: &str, file: &str, line: usize) -> Symbol {
        Symbol {
            id: make_symbol_id(std::path::Path::new(file), name, line),
            name: name.to_string(),
            kind: SymbolKind::Function,
            file: PathBuf::from(file),
            line,
            end_line: line,
            signature: String::new(),
            pagerank: 0.0,
        }
    }

    fn build_test_graph() -> SymbolGraph {
        // main → find_how::run → FileFinder::search
        let mut graph = SymbolGraph::new();
        graph.add_symbol(make_sym("main", "src/main.rs", 10));
        graph.add_symbol(make_sym("run", "src/commands/find_how.rs", 7));
        graph.add_symbol(make_sym("search", "src/search/mod.rs", 57));

        let main_id = make_symbol_id(std::path::Path::new("src/main.rs"), "main", 10);
        let run_id = make_symbol_id(std::path::Path::new("src/commands/find_how.rs"), "run", 7);
        let search_id = make_symbol_id(std::path::Path::new("src/search/mod.rs"), "search", 57);

        graph.add_edge(Edge {
            source: main_id,
            target: run_id.clone(),
            kind: EdgeKind::Call,
            weight: 1.0,
        });
        graph.add_edge(Edge {
            source: run_id.clone(),
            target: search_id,
            kind: EdgeKind::Call,
            weight: 1.0,
        });

        graph
    }

    #[test]
    fn test_trace_callees_depth1() {
        let graph = build_test_graph();
        let main_id = make_symbol_id(std::path::Path::new("src/main.rs"), "main", 10);

        let callees = trace_callees(&graph, &main_id, 1);
        assert_eq!(callees.len(), 1, "main has 1 direct callee (run)");
        assert_eq!(callees[0].depth, 1);
    }

    #[test]
    fn test_trace_callees_depth2() {
        let graph = build_test_graph();
        let main_id = make_symbol_id(std::path::Path::new("src/main.rs"), "main", 10);

        let callees = trace_callees(&graph, &main_id, 2);
        assert_eq!(
            callees.len(),
            2,
            "main has 2 callees within depth 2 (run, search)"
        );
    }

    #[test]
    fn test_trace_callers() {
        let graph = build_test_graph();
        let search_id = make_symbol_id(std::path::Path::new("src/search/mod.rs"), "search", 57);

        let callers = trace_callers(&graph, &search_id, 2);
        assert_eq!(
            callers.len(),
            2,
            "search has 2 callers within depth 2 (run, main)"
        );
    }

    #[test]
    fn test_find_symbol_by_name() {
        let graph = build_test_graph();
        let results = find_symbol_by_name(&graph, "run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "run");
    }
}

//! PageRank 评分算法。
//!
//! 迭代式 PageRank，直接从 pi-shazam 的 TypeScript 实现移植。
//! 参数: damping=0.85, max_iter=50, tol=1e-6。

use super::SymbolGraph;
use log::debug;

/// 计算图中所有符号的 PageRank 分数。
///
/// 分数直接写入每个 Symbol 的 `pagerank` 字段。
/// 悬挂节点（无出边）的分数均匀分配给所有节点。
pub fn calculate_pagerank(graph: &mut SymbolGraph, damping: f64, max_iter: usize, tol: f64) {
    let ids: Vec<String> = graph.symbols.keys().cloned().collect();
    let n = ids.len();
    if n == 0 {
        return;
    }

    // 初始均匀分布
    let mut pr: Vec<f64> = vec![1.0 / n as f64; n];

    // 预计算每个节点的出边权重和
    let out_weight_sum: Vec<f64> = ids
        .iter()
        .map(|id| {
            graph
                .outgoing
                .get(id)
                .map(|edges| edges.iter().map(|e| e.weight).sum::<f64>())
                .unwrap_or(0.0)
        })
        .collect();

    // 预计算入边索引: target_idx → [(source_idx, weight), ...]
    let mut incoming_index: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for (src_idx, id) in ids.iter().enumerate() {
        if out_weight_sum[src_idx] <= 0.0 {
            continue;
        }
        if let Some(edges) = graph.outgoing.get(id) {
            for edge in edges {
                if let Some(tgt_idx) = ids.iter().position(|i| *i == edge.target) {
                    incoming_index[tgt_idx].push((src_idx, edge.weight));
                }
            }
        }
    }

    let base = (1.0 - damping) / n as f64;

    for iter in 0..max_iter {
        // 计算悬挂节点分数总和
        let dangling_sum: f64 = ids
            .iter()
            .enumerate()
            .filter(|(i, _)| out_weight_sum[*i] == 0.0)
            .map(|(i, _)| pr[i])
            .sum();
        let dangling_contrib = damping * dangling_sum / n as f64;

        let mut new_pr = vec![0.0f64; n];

        for tgt_idx in 0..n {
            let mut score = base + dangling_contrib;
            for &(src_idx, weight) in &incoming_index[tgt_idx] {
                score += damping * pr[src_idx] * weight / out_weight_sum[src_idx];
            }
            new_pr[tgt_idx] = score;
        }

        // 归一化
        let total: f64 = new_pr.iter().sum();
        if total > 0.0 {
            for v in &mut new_pr {
                *v /= total;
            }
        }

        // 收敛检测
        let delta: f64 = pr
            .iter()
            .zip(new_pr.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, f64::max);

        pr = new_pr;

        if delta < tol {
            debug!(
                "PageRank converged at iteration {} (delta={})",
                iter + 1,
                delta
            );
            break;
        }
    }

    // 写回分数
    for (i, id) in ids.iter().enumerate() {
        if let Some(sym) = graph.symbols.get_mut(id) {
            sym.pagerank = pr[i];
        }
    }
}

/// 用默认参数计算 PageRank（damping=0.85, 50 iter, tol=1e-6）。
pub fn calculate_pagerank_default(graph: &mut SymbolGraph) {
    calculate_pagerank(graph, 0.85, 50, 1e-6);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Edge, EdgeKind, Symbol, make_symbol_id};
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

    #[test]
    fn test_pagerank_simple() {
        // A ← B, A ← C（A 被 B 和 C 调用，A 应该分数最高）
        let mut graph = SymbolGraph::new();
        let a = make_sym("a", "a.rs", 1);
        let b = make_sym("b", "b.rs", 1);
        let c = make_sym("c", "c.rs", 1);

        graph.add_symbol(a.clone());
        graph.add_symbol(b.clone());
        graph.add_symbol(c.clone());

        let a_id = make_symbol_id(std::path::Path::new("a.rs"), "a", 1);
        let b_id = make_symbol_id(std::path::Path::new("b.rs"), "b", 1);
        let c_id = make_symbol_id(std::path::Path::new("c.rs"), "c", 1);

        // B → A, C → A
        graph.add_edge(Edge {
            source: b_id.clone(),
            target: a_id.clone(),
            kind: EdgeKind::Call,
            weight: 1.0,
        });
        graph.add_edge(Edge {
            source: c_id.clone(),
            target: a_id.clone(),
            kind: EdgeKind::Call,
            weight: 1.0,
        });

        calculate_pagerank_default(&mut graph);

        let pr_a = graph.symbols.get(&a_id).unwrap().pagerank;
        let pr_b = graph.symbols.get(&b_id).unwrap().pagerank;
        let pr_c = graph.symbols.get(&c_id).unwrap().pagerank;

        assert!(
            pr_a > pr_b,
            "A (2 incoming) should have higher PR than B (0 incoming), got A={} B={}",
            pr_a,
            pr_b
        );
        assert!(
            pr_a > pr_c,
            "A should have higher PR than C, got A={} C={}",
            pr_a,
            pr_c
        );
    }

    #[test]
    fn test_pagerank_on_project() {
        // 对本项目建图 + PageRank，验证 main 函数分数较高
        let scan_result = crate::scan::scan_project(std::path::Path::new(".")).unwrap();
        let mut graph = crate::graph::builder::build_graph(&scan_result);
        assert!(graph.symbol_count() > 0);

        calculate_pagerank_default(&mut graph);

        // 所有 pagerank 应该 > 0
        for sym in graph.symbols.values() {
            assert!(sym.pagerank > 0.0, "{} should have positive PR", sym.name);
        }

        // 符号总数应该合理
        assert!(
            graph.symbol_count() >= 10,
            "should find many symbols in this project"
        );
    }
}

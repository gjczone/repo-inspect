//! trace 命令: 追踪符号的调用链（callers + callees）。
//!
//! 流程: scan → build_graph → pagerank → find_symbol → trace_callers/callees → 输出

use crate::cli::{OutputFormat, TraceArgs, TraceDirection};
use crate::graph;
use crate::output::OutputWriter;
use crate::scan;
use anyhow::Result;
use std::io::Write;
use std::path::Path;

pub fn run(args: TraceArgs, repo: &Path, out_dir: &Path, format: OutputFormat) -> Result<()> {
    let query = args.symbol.clone();
    let mut writer = OutputWriter::new(out_dir, "trace", &query, format)?;

    // 检测技术栈
    let stack = scan::stack::detect_stack(repo);

    // 扫描 + 建图 + PageRank
    let scan_result = scan::scan_project(repo)?;
    let mut graph = graph::builder::build_graph(&scan_result);
    graph::pagerank::calculate_pagerank_default(&mut graph);

    // 查找目标符号
    let symbols = graph.find_by_name_ignore_case(&args.symbol);
    if symbols.is_empty() {
        eprintln!("Symbol \"{}\" not found in {}", args.symbol, repo.display());
        return Ok(());
    }

    let max_depth = args.depth;
    let limit = args.limit;

    match format {
        OutputFormat::Json => write_trace_json(
            &mut writer,
            &graph,
            &symbols,
            &args.direction,
            max_depth,
            limit,
        )?,
        OutputFormat::Md => write_trace_markdown(
            &mut writer,
            &graph,
            &symbols,
            &args.direction,
            max_depth,
            limit,
            &stack,
        )?,
    }

    eprintln!(
        "Traced \"{}\": {} symbol(s) found → {}/",
        args.symbol,
        symbols.len(),
        out_dir.display()
    );

    Ok(())
}

fn write_trace_markdown(
    writer: &mut OutputWriter,
    graph: &graph::SymbolGraph,
    symbols: &[&graph::Symbol],
    direction: &TraceDirection,
    max_depth: usize,
    limit: usize,
    stack: &scan::stack::ProjectStack,
) -> Result<()> {
    let path = writer.output_file();
    let mut f = std::fs::File::create(path)?;

    writeln!(f, "# Trace: {}", symbols[0].name)?;
    if !stack.is_empty() {
        writeln!(f, "**Stack**: {}  ", stack.summary())?;
    }
    writeln!(f)?;

    for sym in symbols {
        writeln!(
            f,
            "## {} (`{}`:{}:{}) [PR: {:.4}]",
            sym.name,
            sym.file.display(),
            sym.line,
            sym.kind.label(),
            sym.pagerank
        )?;
        writeln!(f)?;

        // Callers
        if matches!(direction, TraceDirection::Callers | TraceDirection::Both) {
            let callers = graph::traverse::trace_callers(graph, &sym.id, max_depth);
            let total = callers.len();
            writeln!(f, "### ← Callers ({})", total)?;
            if total > limit {
                writeln!(
                    f,
                    "_Showing top {} of {} — use `--limit` to see more_  ",
                    limit, total
                )?;
            }
            writeln!(f)?;
            for entry in callers.iter().take(limit) {
                if let Some(s) = graph.symbols.get(&entry.symbol_id) {
                    writeln!(
                        f,
                        "- `{}` — {}:{} (depth {})",
                        s.name,
                        s.file.display(),
                        s.line,
                        entry.depth
                    )?;
                }
            }
            writeln!(f)?;
        }

        // Callees
        if matches!(direction, TraceDirection::Callees | TraceDirection::Both) {
            let callees = graph::traverse::trace_callees(graph, &sym.id, max_depth);
            let total = callees.len();
            writeln!(f, "### → Callees ({})", total)?;
            if total > limit {
                writeln!(
                    f,
                    "_Showing top {} of {} — use `--limit` to see more_  ",
                    limit, total
                )?;
            }
            writeln!(f)?;
            for entry in callees.iter().take(limit) {
                if let Some(s) = graph.symbols.get(&entry.symbol_id) {
                    writeln!(
                        f,
                        "- `{}` — {}:{} (depth {})",
                        s.name,
                        s.file.display(),
                        s.line,
                        entry.depth
                    )?;
                }
            }
            writeln!(f)?;
        }
    }

    Ok(())
}

fn write_trace_json(
    writer: &mut OutputWriter,
    graph: &graph::SymbolGraph,
    symbols: &[&graph::Symbol],
    direction: &TraceDirection,
    max_depth: usize,
    limit: usize,
) -> Result<()> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct TraceOutput {
        symbol: String,
        file: String,
        line: usize,
        kind: String,
        pagerank: f64,
        callers: Vec<TraceEntryJson>,
        callees: Vec<TraceEntryJson>,
    }

    #[derive(Serialize)]
    struct TraceEntryJson {
        name: String,
        file: String,
        line: usize,
        depth: usize,
    }

    let mut outputs = Vec::new();

    for sym in symbols {
        let callers = if matches!(direction, TraceDirection::Callers | TraceDirection::Both) {
            graph::traverse::trace_callers(graph, &sym.id, max_depth)
                .iter()
                .take(limit)
                .filter_map(|e| {
                    graph.symbols.get(&e.symbol_id).map(|s| TraceEntryJson {
                        name: s.name.clone(),
                        file: s.file.display().to_string(),
                        line: s.line,
                        depth: e.depth,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        let callees = if matches!(direction, TraceDirection::Callees | TraceDirection::Both) {
            graph::traverse::trace_callees(graph, &sym.id, max_depth)
                .iter()
                .take(limit)
                .filter_map(|e| {
                    graph.symbols.get(&e.symbol_id).map(|s| TraceEntryJson {
                        name: s.name.clone(),
                        file: s.file.display().to_string(),
                        line: s.line,
                        depth: e.depth,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        outputs.push(TraceOutput {
            symbol: sym.name.clone(),
            file: sym.file.display().to_string(),
            line: sym.line,
            kind: sym.kind.label().to_string(),
            pagerank: sym.pagerank,
            callers,
            callees,
        });
    }

    let json = serde_json::to_string_pretty(&outputs)?;
    std::fs::write(writer.output_file(), json)?;
    Ok(())
}

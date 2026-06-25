//! hotspots 命令: 按 PageRank 排序的热点符号。
//!
//! 流程: scan → build_graph → pagerank → top_symbols → 输出

use crate::cli::{HotspotsArgs, OutputFormat};
use crate::graph;
use crate::output::OutputWriter;
use crate::scan;
use anyhow::Result;
use std::io::Write;
use std::path::Path;

pub fn run(args: HotspotsArgs, repo: &Path, out_dir: &Path, format: OutputFormat) -> Result<()> {
    let writer = OutputWriter::new(out_dir, "hotspots", "", format)?;

    let scan_result = scan::scan_project(repo)?;
    let mut graph = graph::builder::build_graph(&scan_result);
    graph::pagerank::calculate_pagerank_default(&mut graph);

    let top = graph.top_symbols(args.count);

    match format {
        OutputFormat::Json => write_json(&writer, &top, &graph)?,
        OutputFormat::Md => write_markdown(&writer, &top, &graph)?,
    }

    eprintln!(
        "Top {} hotspots (of {} symbols) → {}/",
        top.len(),
        graph.symbol_count(),
        out_dir.display()
    );

    Ok(())
}

fn write_markdown(
    writer: &OutputWriter,
    top: &[&graph::Symbol],
    graph: &graph::SymbolGraph,
) -> Result<()> {
    let mut f = std::fs::File::create(writer.output_file())?;

    writeln!(f, "# Hotspots — Top {} Symbols by PageRank", top.len())?;
    writeln!(f)?;
    writeln!(f, "| Rank | Symbol | Kind | File:Line | PageRank | Edges |")?;
    writeln!(f, "|------|--------|------|-----------|----------|-------|")?;

    for (i, sym) in top.iter().enumerate() {
        let out_count = graph.outgoing.get(&sym.id).map(|e| e.len()).unwrap_or(0);
        let in_count = graph.incoming.get(&sym.id).map(|e| e.len()).unwrap_or(0);

        writeln!(
            f,
            "| {} | `{}` | {} | {}:{} | {:.4} | {}in/{}out |",
            i + 1,
            sym.name,
            sym.kind.label(),
            sym.file.display(),
            sym.line,
            sym.pagerank,
            in_count,
            out_count
        )?;
    }

    Ok(())
}

fn write_json(
    writer: &OutputWriter,
    top: &[&graph::Symbol],
    graph: &graph::SymbolGraph,
) -> Result<()> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct HotspotEntry {
        rank: usize,
        name: String,
        kind: String,
        file: String,
        line: usize,
        pagerank: f64,
        incoming_edges: usize,
        outgoing_edges: usize,
    }

    let entries: Vec<HotspotEntry> = top
        .iter()
        .enumerate()
        .map(|(i, sym)| HotspotEntry {
            rank: i + 1,
            name: sym.name.clone(),
            kind: sym.kind.label().to_string(),
            file: sym.file.display().to_string(),
            line: sym.line,
            pagerank: sym.pagerank,
            incoming_edges: graph.incoming.get(&sym.id).map(|e| e.len()).unwrap_or(0),
            outgoing_edges: graph.outgoing.get(&sym.id).map(|e| e.len()).unwrap_or(0),
        })
        .collect();

    let json = serde_json::to_string_pretty(&entries)?;
    std::fs::write(writer.output_file(), json)?;
    Ok(())
}

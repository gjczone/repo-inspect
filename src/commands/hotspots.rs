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

/// 常见内置/标准库函数名 — 排除在 hotspots 之外。
///
/// 这些名称在任何语言项目中都是高频调用目标，出现在 hotspots 中没有分析价值。
/// 覆盖 JS/TS、Python、Rust、Go 的常见内置方法和函数。
const BUILTIN_NAMES: &[&str] = &[
    // JS/TS 内置方法
    "toString",
    "valueOf",
    "hasOwnProperty",
    "toLocaleString",
    "then",
    "catch",
    "finally",
    "map",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "forEach",
    "reduce",
    "reduceRight",
    "some",
    "every",
    "includes",
    "indexOf",
    "lastIndexOf",
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "slice",
    "concat",
    "join",
    "sort",
    "reverse",
    "fill",
    "copyWithin",
    "flat",
    "flatMap",
    "keys",
    "values",
    "entries",
    "from",
    "of",
    "isArray",
    "startsWith",
    "endsWith",
    "includes",
    "repeat",
    "trim",
    "trimStart",
    "trimEnd",
    "padStart",
    "padEnd",
    "replace",
    "replaceAll",
    "split",
    "match",
    "matchAll",
    "search",
    "substring",
    "slice",
    "charAt",
    "charCodeAt",
    "at",
    "toLowerCase",
    "toUpperCase",
    "localeCompare",
    "apply",
    "bind",
    "call",
    "assign",
    "create",
    "defineProperty",
    "defineProperties",
    "getOwnPropertyDescriptor",
    "getOwnPropertyNames",
    "getPrototypeOf",
    "setPrototypeOf",
    "is",
    "freeze",
    "seal",
    "preventExtensions",
    "parse",
    "stringify",
    "setTimeout",
    "setInterval",
    "clearTimeout",
    "clearInterval",
    "addEventListener",
    "removeEventListener",
    "dispatchEvent",
    "querySelector",
    "querySelectorAll",
    "getElementById",
    "getElementsByClassName",
    "setAttribute",
    "getAttribute",
    "removeAttribute",
    "log",
    "warn",
    "error",
    "info",
    "debug",
    "next",
    "return",
    "throw",
    "test",
    "exec",
    "max",
    "min",
    "abs",
    "ceil",
    "floor",
    "round",
    "sqrt",
    "pow",
    "random",
    "now",
    // Python 内置
    "print",
    "len",
    "range",
    "enumerate",
    "zip",
    "map",
    "filter",
    "sorted",
    "reversed",
    "isinstance",
    "issubclass",
    "hasattr",
    "getattr",
    "setattr",
    "delattr",
    "str",
    "int",
    "float",
    "bool",
    "list",
    "dict",
    "set",
    "tuple",
    "bytes",
    "type",
    "object",
    "super",
    "property",
    "classmethod",
    "staticmethod",
    "open",
    "input",
    "format",
    "repr",
    "hash",
    "id",
    "dir",
    "vars",
    "help",
    "min",
    "max",
    "abs",
    "sum",
    "any",
    "all",
    "round",
    "pow",
    "divmod",
    "iter",
    "next",
    "callable",
    "iter",
    "next",
    "append",
    "extend",
    "insert",
    "remove",
    "pop",
    "clear",
    "copy",
    "sort",
    "reverse",
    "update",
    "get",
    "setdefault",
    "popitem",
    "keys",
    "values",
    "items",
    "encode",
    "decode",
    "strip",
    "lstrip",
    "rstrip",
    "split",
    "join",
    "replace",
    "startswith",
    "endswith",
    "upper",
    "lower",
    "title",
    "capitalize",
    "read",
    "write",
    "readline",
    "readlines",
    "close",
    "flush",
    // Rust 标准库高频方法
    "unwrap",
    "expect",
    "ok",
    "err",
    "map",
    "and_then",
    "or_else",
    "unwrap_or",
    "unwrap_or_else",
    "unwrap_or_default",
    "map_err",
    "is_some",
    "is_none",
    "is_ok",
    "is_err",
    "to_string",
    "to_owned",
    "clone",
    "default",
    "from",
    "into",
    "fmt",
    "display",
    "debug",
    "len",
    "is_empty",
    "contains",
    "push",
    "pop",
    "insert",
    "remove",
    "clear",
    "iter",
    "enumerate",
    "zip",
    "chain",
    "collect",
    "fold",
    "reduce",
    "filter",
    "map",
    "for_each",
    "any",
    "all",
    "find",
    "position",
    "sort",
    "sort_by",
    "dedup",
    "retain",
    "split_at",
    "write",
    "writeln",
    "read",
    "read_line",
    "new",
    "build",
    "create",
    // Go 标准库
    "append",
    "len",
    "cap",
    "make",
    "new",
    "delete",
    "close",
    "Errorf",
    "Sprintf",
    "Printf",
    "Fprintf",
    "Println",
    "Fprintln",
    "String",
    "Error",
    "Write",
    "Read",
    "len",
    "append",
    "copy",
    "delete",
];

/// 判断符号名是否为常见内置/标准库函数。
fn is_builtin_name(name: &str) -> bool {
    BUILTIN_NAMES.contains(&name)
}

pub fn run(args: HotspotsArgs, repo: &Path, out_dir: &Path, format: OutputFormat) -> Result<()> {
    let writer = OutputWriter::new(out_dir, "hotspots", "", format)?;

    let scan_result = scan::scan_project(repo)?;
    let mut graph = graph::builder::build_graph(&scan_result);
    graph::pagerank::calculate_pagerank_default(&mut graph);

    // 多取一些符号以补偿内置函数过滤，确保最终数量够用
    let raw_top = graph.top_symbols(args.count * 3);
    // 过滤掉常见内置/标准库函数名 — 这些在 hotspots 中没有分析价值
    let top: Vec<&graph::Symbol> = raw_top
        .into_iter()
        .filter(|sym| !is_builtin_name(&sym.name))
        .take(args.count)
        .collect();

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

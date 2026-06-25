//! data 命令: 提取核心数据结构（struct/enum/interface/type/class）。
//!
//! 流程: scan → 过滤 type-kind 符号 → 按名称过滤（可选）→ 输出

use crate::cli::{DataArgs, OutputFormat};
use crate::output::OutputWriter;
use crate::scan;
use crate::scan::parser::SymbolKind;
use anyhow::Result;
use std::io::Write;
use std::path::Path;

/// 只保留类型定义类符号。
fn is_type_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Struct
            | SymbolKind::Enum
            | SymbolKind::Interface
            | SymbolKind::TypeAlias
            | SymbolKind::Class
    )
}

pub fn run(args: DataArgs, repo: &Path, out_dir: &Path, format: OutputFormat) -> Result<()> {
    // 有名称过滤时作为文件名后缀（如 data-RepoSpec.md），无过滤时省略（如 data.md）
    let query_label = args.name.as_deref().unwrap_or("");
    let writer = OutputWriter::new(out_dir, "data", query_label, format)?;

    let scan_result = scan::scan_project(repo)?;

    // 收集所有类型定义
    let mut types: Vec<(&std::path::Path, &scan::parser::ExtractedSymbol)> = Vec::new();
    for file in &scan_result.files {
        for sym in &file.symbols {
            if is_type_kind(sym.kind) {
                // 按名称过滤（如果指定）
                if let Some(ref name_filter) = args.name
                    && !sym
                        .name
                        .to_lowercase()
                        .contains(&name_filter.to_lowercase())
                {
                    continue;
                }
                types.push((file.path.as_path(), sym));
            }
        }
    }

    match format {
        OutputFormat::Json => write_json(&writer, &types)?,
        OutputFormat::Md => write_markdown(&writer, &types)?,
    }

    eprintln!(
        "Found {} type definitions → {}/",
        types.len(),
        out_dir.display()
    );

    Ok(())
}

fn write_markdown(
    writer: &OutputWriter,
    types: &[(&std::path::Path, &scan::parser::ExtractedSymbol)],
) -> Result<()> {
    let mut f = std::fs::File::create(writer.output_file())?;

    writeln!(f, "# Data Structures")?;
    writeln!(f)?;
    writeln!(f, "**Found**: {} type definitions  ", types.len())?;
    writeln!(f)?;

    if types.is_empty() {
        writeln!(f, "_No type definitions found._")?;
        return Ok(());
    }

    // 按 kind 分组
    let mut by_kind: std::collections::BTreeMap<&str, Vec<_>> = std::collections::BTreeMap::new();
    for (path, sym) in types {
        by_kind
            .entry(sym.kind.label())
            .or_default()
            .push((path, sym));
    }

    for (kind, syms) in &by_kind {
        writeln!(f, "## {}s ({})  ", kind, syms.len())?;
        writeln!(f)?;
        for (path, sym) in syms {
            writeln!(
                f,
                "- **{}** `{}` — {}:{}  ",
                kind,
                sym.name,
                path.display(),
                sym.line
            )?;
            let sig = sym.signature.trim();
            if !sig.is_empty() {
                writeln!(f, "  `{}`  ", sig)?;
            }
        }
        writeln!(f)?;
    }

    Ok(())
}

fn write_json(
    writer: &OutputWriter,
    types: &[(&std::path::Path, &scan::parser::ExtractedSymbol)],
) -> Result<()> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct TypeEntry {
        name: String,
        kind: String,
        file: String,
        line: usize,
        end_line: usize,
        signature: String,
    }

    let entries: Vec<TypeEntry> = types
        .iter()
        .map(|(path, sym)| TypeEntry {
            name: sym.name.clone(),
            kind: sym.kind.label().to_string(),
            file: path.display().to_string(),
            line: sym.line,
            end_line: sym.end_line,
            signature: sym.signature.clone(),
        })
        .collect();

    let json = serde_json::to_string_pretty(&entries)?;
    std::fs::write(writer.output_file(), json)?;
    Ok(())
}

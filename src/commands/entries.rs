//! entries 命令: 检测项目入口点。
//!
//! 检测模式:
//! - CLI: fn main(), #[tokio::main]
//! - HTTP: #[get("/...)], app.get("/..."), @router
//! - Event: .on(, .subscribe(

use crate::cli::{EntriesArgs, OutputFormat};
use crate::output::OutputWriter;
use crate::scan;
use crate::scan::parser::SymbolKind;
use anyhow::Result;
use std::io::Write;
use std::path::Path;

/// 入口点类型。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum EntryKind {
    Cli,
    Http,
    Event,
}

impl EntryKind {
    fn label(&self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Http => "http",
            Self::Event => "event",
        }
    }
}

/// 检测到的入口点。
struct EntryPoint {
    kind: EntryKind,
    name: String,
    file: std::path::PathBuf,
    line: usize,
    signature: String,
}

pub fn run(args: EntriesArgs, repo: &Path, out_dir: &Path, format: OutputFormat) -> Result<()> {
    let writer = OutputWriter::new(out_dir, "entries", "", format)?;

    let scan_result = scan::scan_project(repo)?;
    let mut entries: Vec<EntryPoint> = Vec::new();

    // 过滤器
    let kind_filter = args.kind.to_lowercase();

    for file in &scan_result.files {
        // CLI 入口: fn main()
        for sym in &file.symbols {
            if sym.name == "main"
                && sym.kind == SymbolKind::Function
                && (kind_filter == "all" || kind_filter == "cli")
            {
                entries.push(EntryPoint {
                    kind: EntryKind::Cli,
                    name: "main".to_string(),
                    file: file.path.clone(),
                    line: sym.line,
                    signature: sym.signature.clone(),
                });
            }
        }

        // HTTP 路由: 扫描 calls 中的 post/put/patch/route
        // 注意: get/delete 太通用（Map.get, Set.delete），不作为 HTTP 入口指标
        for call in &file.calls {
            let name = call.name.to_lowercase();
            if matches!(name.as_str(), "post" | "put" | "patch" | "route")
                && (kind_filter == "all" || kind_filter == "http")
            {
                entries.push(EntryPoint {
                    kind: EntryKind::Http,
                    name: call.name.clone(),
                    file: file.path.clone(),
                    line: call.line,
                    signature: String::new(),
                });
            }
        }

        // Event: .subscribe(, .addEventListener, .emit
        // 注意: on 太通用（EventEmitter.on 等），不作为事件入口指标
        for call in &file.calls {
            let name = call.name.to_lowercase();
            if matches!(name.as_str(), "subscribe" | "addeventlistener" | "emit")
                && (kind_filter == "all" || kind_filter == "event")
            {
                entries.push(EntryPoint {
                    kind: EntryKind::Event,
                    name: call.name.clone(),
                    file: file.path.clone(),
                    line: call.line,
                    signature: String::new(),
                });
            }
        }
    }

    match format {
        OutputFormat::Json => write_json(&writer, &entries)?,
        OutputFormat::Md => write_markdown(&writer, &entries)?,
    }

    eprintln!(
        "Found {} entry points → {}/",
        entries.len(),
        out_dir.display()
    );

    Ok(())
}

fn write_markdown(writer: &OutputWriter, entries: &[EntryPoint]) -> Result<()> {
    let mut f = std::fs::File::create(writer.output_file())?;

    writeln!(f, "# Entry Points")?;
    writeln!(f)?;
    writeln!(f, "**Found**: {}  ", entries.len())?;
    writeln!(f)?;

    if entries.is_empty() {
        writeln!(f, "_No entry points detected._")?;
        return Ok(());
    }

    // 按类型分组
    let mut by_kind: std::collections::BTreeMap<&str, Vec<&EntryPoint>> =
        std::collections::BTreeMap::new();
    for ep in entries {
        by_kind.entry(ep.kind.label()).or_default().push(ep);
    }

    for (kind, eps) in &by_kind {
        writeln!(f, "## {} ({})  ", kind, eps.len())?;
        writeln!(f)?;
        for ep in eps.iter().take(30) {
            if ep.signature.is_empty() {
                writeln!(f, "- `{}` — {}:{}  ", ep.name, ep.file.display(), ep.line)?;
            } else {
                writeln!(f, "- `{}` — {}:{}  ", ep.name, ep.file.display(), ep.line)?;
                writeln!(f, "  `{}`  ", ep.signature.trim())?;
            }
        }
        writeln!(f)?;
    }

    Ok(())
}

fn write_json(writer: &OutputWriter, entries: &[EntryPoint]) -> Result<()> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct EntryJson {
        kind: String,
        name: String,
        file: String,
        line: usize,
    }

    let items: Vec<EntryJson> = entries
        .iter()
        .map(|ep| EntryJson {
            kind: ep.kind.label().to_string(),
            name: ep.name.clone(),
            file: ep.file.display().to_string(),
            line: ep.line,
        })
        .collect();

    let json = serde_json::to_string_pretty(&items)?;
    std::fs::write(writer.output_file(), json)?;
    Ok(())
}

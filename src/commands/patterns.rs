//! patterns 命令: 检测项目中的设计模式和惯用法。
//!
//! 当前支持的类别:
//! - concurrency: async fn, Mutex, RwLock, Arc, channel, tokio::spawn
//! - structural: trait impl, module 组织
//! - all: 以上全部

use crate::cli::{OutputFormat, PatternsArgs};
use crate::output::OutputWriter;
use crate::scan;
use crate::scan::parser::SymbolKind;
use anyhow::Result;
use std::io::Write;
use std::path::Path;

/// 检测到的模式实例。
struct PatternInstance {
    category: String,
    pattern: String,
    file: std::path::PathBuf,
    line: usize,
    detail: String,
}

pub fn run(args: PatternsArgs, repo: &Path, out_dir: &Path, format: OutputFormat) -> Result<()> {
    let category_filter = args.category.as_deref().unwrap_or("all");
    let writer = OutputWriter::new(out_dir, "patterns", category_filter, format)?;

    let scan_result = scan::scan_project(repo)?;
    let mut patterns: Vec<PatternInstance> = Vec::new();

    for file in &scan_result.files {
        // 检测 concurrency 模式
        if category_filter == "all" || category_filter == "concurrency" {
            detect_concurrency(file, &mut patterns);
        }

        // 检测 structural 模式
        if category_filter == "all" || category_filter == "structural" {
            detect_structural(file, &mut patterns);
        }
    }

    match format {
        OutputFormat::Json => write_json(&writer, &patterns)?,
        OutputFormat::Md => write_markdown(&writer, &patterns)?,
    }

    eprintln!(
        "Detected {} pattern instances → {}/",
        patterns.len(),
        out_dir.display()
    );

    Ok(())
}

/// 检测并发模式: async fn, Mutex, RwLock, Arc, channel, spawn。
fn detect_concurrency(file: &scan::parser::ParsedFile, patterns: &mut Vec<PatternInstance>) {
    for sym in &file.symbols {
        let sig = sym.signature.to_lowercase();

        if sig.contains("async ") {
            patterns.push(PatternInstance {
                category: "concurrency".to_string(),
                pattern: "async function".to_string(),
                file: file.path.clone(),
                line: sym.line,
                detail: sym.name.clone(),
            });
        }

        if sig.contains("mutex") || sig.contains("rwlock") {
            patterns.push(PatternInstance {
                category: "concurrency".to_string(),
                pattern: "lock pattern".to_string(),
                file: file.path.clone(),
                line: sym.line,
                detail: sym.name.clone(),
            });
        }
    }

    // 检测 calls 中的并发原语
    for call in &file.calls {
        let name = call.name.as_str();
        if matches!(
            name,
            "spawn" | "block_on" | "spawn_blocking" | "channel" | "mpsc"
        ) {
            patterns.push(PatternInstance {
                category: "concurrency".to_string(),
                pattern: format!("{}()", name),
                file: file.path.clone(),
                line: call.line,
                detail: call.name.clone(),
            });
        }
    }
}

/// 检测结构模式: trait impl, mod 声明, 重导出。
fn detect_structural(file: &scan::parser::ParsedFile, patterns: &mut Vec<PatternInstance>) {
    for sym in &file.symbols {
        match sym.kind {
            SymbolKind::Trait => {
                patterns.push(PatternInstance {
                    category: "structural".to_string(),
                    pattern: "trait definition".to_string(),
                    file: file.path.clone(),
                    line: sym.line,
                    detail: sym.name.clone(),
                });
            }
            SymbolKind::Impl => {
                patterns.push(PatternInstance {
                    category: "structural".to_string(),
                    pattern: "trait implementation".to_string(),
                    file: file.path.clone(),
                    line: sym.line,
                    detail: sym.name.clone(),
                });
            }
            SymbolKind::Module => {
                patterns.push(PatternInstance {
                    category: "structural".to_string(),
                    pattern: "module declaration".to_string(),
                    file: file.path.clone(),
                    line: sym.line,
                    detail: sym.name.clone(),
                });
            }
            _ => {}
        }
    }
}

fn write_markdown(writer: &OutputWriter, patterns: &[PatternInstance]) -> Result<()> {
    let mut f = std::fs::File::create(writer.output_file())?;

    writeln!(f, "# Patterns")?;
    writeln!(f)?;
    writeln!(f, "**Detected**: {} instances  ", patterns.len())?;
    writeln!(f)?;

    if patterns.is_empty() {
        writeln!(f, "_No patterns detected._")?;
        return Ok(());
    }

    // 按 category → pattern 分组
    let mut by_cat: std::collections::BTreeMap<
        &str,
        std::collections::BTreeMap<&str, Vec<&PatternInstance>>,
    > = std::collections::BTreeMap::new();

    for p in patterns {
        by_cat
            .entry(p.category.as_str())
            .or_default()
            .entry(p.pattern.as_str())
            .or_default()
            .push(p);
    }

    for (cat, pattern_map) in &by_cat {
        writeln!(f, "## {}  ", cat)?;
        writeln!(f)?;
        for (pat, instances) in pattern_map {
            writeln!(f, "### {} ({})  ", pat, instances.len())?;
            writeln!(f)?;
            for inst in instances.iter().take(20) {
                writeln!(
                    f,
                    "- `{}` — {}:{}  ",
                    inst.detail,
                    inst.file.display(),
                    inst.line
                )?;
            }
            if instances.len() > 20 {
                writeln!(f, "- _... and {} more_  ", instances.len() - 20)?;
            }
            writeln!(f)?;
        }
    }

    Ok(())
}

fn write_json(writer: &OutputWriter, patterns: &[PatternInstance]) -> Result<()> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct PatternJson {
        category: String,
        pattern: String,
        file: String,
        line: usize,
        detail: String,
    }

    let items: Vec<PatternJson> = patterns
        .iter()
        .map(|p| PatternJson {
            category: p.category.clone(),
            pattern: p.pattern.clone(),
            file: p.file.display().to_string(),
            line: p.line,
            detail: p.detail.clone(),
        })
        .collect();

    let json = serde_json::to_string_pretty(&items)?;
    std::fs::write(writer.output_file(), json)?;
    Ok(())
}

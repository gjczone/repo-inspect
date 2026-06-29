use crate::cli::OutputFormat;
use crate::scan::parser::{self, ExtractedSymbol};
use crate::search::FileMatch;
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// L1 文本搜索 JSON 输出
#[derive(Serialize)]
struct FindHowOutput {
    query: String,
    files_found: usize,
    results: Vec<FileResult>,
}

/// L2 符号搜索 JSON 输出
#[derive(Serialize)]
struct SymbolFindHowOutput {
    query: String,
    definitions: Vec<SymbolEntry>,
    call_references: Vec<CallEntry>,
}

#[derive(Serialize)]
struct SymbolEntry {
    name: String,
    kind: String,
    file: String,
    line: usize,
    end_line: usize,
    signature: String,
}

#[derive(Serialize)]
struct CallEntry {
    name: String,
    file: String,
    line: usize,
}

#[derive(Serialize)]
struct FileResult {
    path: String,
    score: f64,
    matches: Vec<MatchedLineOutput>,
}

#[derive(Serialize)]
struct MatchedLineOutput {
    line: usize,
    content: String,
    context: String,
}

/// OutputWriter manages writing inspection results to the `.inspect/` directory
pub struct OutputWriter {
    #[allow(dead_code)]
    out_dir: PathBuf,
    output_file: PathBuf,
    format: OutputFormat,
}

impl OutputWriter {
    /// Create a new output writer.
    ///
    /// Creates `.inspect/` directory if it doesn't exist.
    /// Output file: `.inspect/{command}-{sanitized_query}.{ext}`
    pub fn new(out_dir: &Path, command: &str, query: &str, format: OutputFormat) -> Result<Self> {
        fs::create_dir_all(out_dir)?;

        let sanitized = sanitize_filename(query);
        let ext = match format {
            OutputFormat::Json => "json",
            OutputFormat::Md => "md",
        };
        // 无用户查询时输出 `{command}.{ext}`（如 entries.md），有查询时保留后缀（如 data-RepoSpec.md）
        let filename = if sanitized.is_empty() {
            format!("{}.{}", command, ext)
        } else {
            format!("{}-{}.{}", command, sanitized, ext)
        };
        let output_file = out_dir.join(filename);

        Ok(Self {
            out_dir: out_dir.to_path_buf(),
            output_file,
            format,
        })
    }

    /// 获取输出文件路径。
    pub fn output_file(&self) -> &Path {
        &self.output_file
    }

    /// Write inspection results in the configured format
    pub fn write_results(&mut self, matches: &[FileMatch]) -> Result<()> {
        match self.format {
            OutputFormat::Json => self.write_json(matches),
            OutputFormat::Md => self.write_markdown(matches),
        }
    }

    /// Write L2 符号搜索结果：区分定义和调用引用。
    pub fn write_symbol_results(
        &mut self,
        definitions: &[(&Path, &ExtractedSymbol)],
        call_refs: &[(&Path, &parser::CallRef)],
    ) -> Result<()> {
        match self.format {
            OutputFormat::Json => self.write_symbol_json(definitions, call_refs),
            OutputFormat::Md => self.write_symbol_markdown(definitions, call_refs),
        }
    }

    fn write_json(&mut self, matches: &[FileMatch]) -> Result<()> {
        let query = self
            .output_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let output = FindHowOutput {
            query: query.to_string(),
            files_found: matches.len(),
            results: matches
                .iter()
                .map(|m| FileResult {
                    path: m.path.to_string_lossy().to_string(),
                    score: m.score,
                    matches: m
                        .matching_lines
                        .iter()
                        .map(|l| MatchedLineOutput {
                            line: l.line_number,
                            content: l.content.clone(),
                            context: l.context.clone(),
                        })
                        .collect(),
                })
                .collect(),
        };

        let json = serde_json::to_string_pretty(&output)?;
        fs::write(&self.output_file, json)?;
        Ok(())
    }

    fn write_markdown(&self, matches: &[FileMatch]) -> Result<()> {
        let mut f = fs::File::create(&self.output_file)?;
        let query = self
            .output_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("inspection");

        writeln!(f, "# Inspection: {}", query)?;
        writeln!(f)?;
        writeln!(f, "**Files found**: {}  ", matches.len())?;
        writeln!(f)?;

        if matches.is_empty() {
            writeln!(f, "_No matching files found._")?;
            return Ok(());
        }

        // Group results by directory for readability
        let mut by_dir: std::collections::BTreeMap<String, Vec<&FileMatch>> =
            std::collections::BTreeMap::new();
        for m in matches {
            let dir = m
                .path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "(root)".to_string());
            by_dir.entry(dir).or_default().push(m);
        }

        for (dir, files) in &by_dir {
            let best = files
                .iter()
                .map(|f| {
                    f.path
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(f, "### {}/  ", dir)?;
            writeln!(f, "_Key files: {}_  ", best)?;
            writeln!(f)?;

            for mf in files.iter().take(5) {
                // Top 5 per directory
                let fname = mf
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                writeln!(f, "#### `{}`  ", fname)?;
                writeln!(f)?;

                for ml in &mf.matching_lines {
                    writeln!(f, "L{}: `{}`  ", ml.line_number, ml.content.trim())?;
                }
                writeln!(f)?;
            }
        }

        Ok(())
    }

    fn write_symbol_json(
        &mut self,
        definitions: &[(&Path, &ExtractedSymbol)],
        call_refs: &[(&Path, &parser::CallRef)],
    ) -> Result<()> {
        let query = self
            .output_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let output = SymbolFindHowOutput {
            query: query.to_string(),
            definitions: definitions
                .iter()
                .map(|(path, sym)| SymbolEntry {
                    name: sym.name.clone(),
                    kind: format!("{:?}", sym.kind).to_lowercase(),
                    file: path.to_string_lossy().to_string(),
                    line: sym.line,
                    end_line: sym.end_line,
                    signature: sym.signature.clone(),
                })
                .collect(),
            call_references: call_refs
                .iter()
                .map(|(path, call)| CallEntry {
                    name: call.name.clone(),
                    file: path.to_string_lossy().to_string(),
                    line: call.line,
                })
                .collect(),
        };

        let json = serde_json::to_string_pretty(&output)?;
        fs::write(&self.output_file, json)?;
        Ok(())
    }

    fn write_symbol_markdown(
        &self,
        definitions: &[(&Path, &ExtractedSymbol)],
        call_refs: &[(&Path, &parser::CallRef)],
    ) -> Result<()> {
        let mut f = fs::File::create(&self.output_file)?;
        let query = self
            .output_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("inspection");

        writeln!(f, "# Inspection: {}", query)?;
        writeln!(f)?;

        // 定义
        writeln!(f, "## Symbol Definitions ({})  ", definitions.len())?;
        writeln!(f)?;
        if definitions.is_empty() {
            writeln!(f, "_No symbol definitions matched._  ")?;
        }
        for (path, sym) in definitions {
            writeln!(
                f,
                "- **{}** `{}` — {}:{}  ",
                sym.kind.label(),
                sym.name,
                path.display(),
                sym.line
            )?;
            // 签名预览
            let sig_trimmed = sym.signature.trim();
            if !sig_trimmed.is_empty() {
                writeln!(f, "  `{}`  ", sig_trimmed)?;
            }
        }
        writeln!(f)?;

        // 调用引用
        writeln!(f, "## Call References ({})  ", call_refs.len())?;
        writeln!(f)?;
        if call_refs.is_empty() {
            writeln!(f, "_No call references found._  ")?;
        }
        // 按文件分组
        let mut by_file: std::collections::BTreeMap<String, Vec<&parser::CallRef>> =
            std::collections::BTreeMap::new();
        for (path, call) in call_refs {
            by_file
                .entry(path.to_string_lossy().to_string())
                .or_default()
                .push(call);
        }
        for (file, calls) in &by_file {
            writeln!(f, "### {}  ", file)?;
            for call in calls.iter().take(20) {
                writeln!(f, "- L{}: `{}`  ", call.line, call.name)?;
            }
            if calls.len() > 20 {
                writeln!(f, "- _... and {} more_  ", calls.len() - 20)?;
            }
        }

        Ok(())
    }
}

/// Sanitize a string for use as a filename
fn sanitize_filename(s: &str) -> String {
    let sanitized: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    // Truncate to reasonable length (Unicode-safe char iteration)
    if sanitized.chars().count() > 60 {
        sanitized.chars().take(60).collect()
    } else {
        sanitized
    }
}

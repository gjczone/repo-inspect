use crate::cli::OutputFormat;
use crate::search::FileMatch;
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Structured output for JSON format
#[derive(Serialize)]
struct FindHowOutput {
    query: String,
    files_found: usize,
    results: Vec<FileResult>,
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
    pub fn new(
        out_dir: &Path,
        command: &str,
        query: &str,
        format: OutputFormat,
    ) -> Result<Self> {
        fs::create_dir_all(out_dir)?;

        let sanitized = sanitize_filename(query);
        let ext = match format {
            OutputFormat::Json => "json",
            OutputFormat::Md => "md",
        };
        let filename = format!("{}-{}.{}", command, sanitized, ext);
        let output_file = out_dir.join(filename);

        Ok(Self {
            out_dir: out_dir.to_path_buf(),
            output_file,
            format,
        })
    }

    /// Write inspection results in the configured format
    pub fn write_results(&mut self, matches: &[FileMatch]) -> Result<()> {
        match self.format {
            OutputFormat::Json => self.write_json(matches),
            OutputFormat::Md => self.write_markdown(matches),
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
    // Truncate to reasonable length
    if sanitized.len() > 60 {
        sanitized[..60].to_string()
    } else {
        sanitized
    }
}

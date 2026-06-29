use crate::cli::{FindHowArgs, OutputFormat};
use crate::output::OutputWriter;
use crate::scan;
use crate::search::FileFinder;
use anyhow::Result;
use log::debug;
use std::path::Path;

pub fn run(args: FindHowArgs, repo: &Path, out_dir: &Path, format: OutputFormat) -> Result<()> {
    let query = args.query.join(" ");
    let mut writer = OutputWriter::new(out_dir, "find-how", &query, format)?;

    // L2: 尝试 tree-sitter 符号搜索
    match scan::scan_project(repo) {
        Ok(scan_result) => {
            let definitions = scan::find_symbols(&scan_result, &query);
            let call_refs = scan::find_call_refs(&scan_result, &query);

            if !definitions.is_empty() || !call_refs.is_empty() {
                // L2 命中：输出符号定义 + 调用引用
                debug!(
                    "L2 hit: {} definitions, {} call references for \"{}\"",
                    definitions.len(),
                    call_refs.len(),
                    query
                );

                writer.write_symbol_results(&definitions, &call_refs)?;

                eprintln!(
                    "Found {} definitions + {} call references for \"{}\" → {}/",
                    definitions.len(),
                    call_refs.len(),
                    query,
                    out_dir.display()
                );
                return Ok(());
            }

            debug!("L2 no symbol matches for \"{}\", falling back to L1", query);
        }
        Err(e) => {
            debug!("L2 scan failed ({}), falling back to L1", e);
        }
    }

    // L1 fallback: 纯文本搜索
    let finder = FileFinder::new(repo)?;
    let matches = finder.search(&query, args.depth)?;

    writer.write_results(&matches)?;

    eprintln!(
        "Found {} relevant files for \"{}\" → {}/",
        matches.len(),
        query,
        out_dir.display()
    );

    Ok(())
}

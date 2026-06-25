use crate::cli::{FindHowArgs, OutputFormat};
use crate::output::OutputWriter;
use crate::search::FileFinder;
use anyhow::Result;
use std::path::Path;

pub fn run(args: FindHowArgs, repo: &Path, out_dir: &Path, format: OutputFormat) -> Result<()> {
    let query = args.query.join(" ");

    let mut writer = OutputWriter::new(out_dir, "find-how", &query, format)?;
    let finder = FileFinder::new(repo)?;
    let matches = finder.search(&query, args.depth);

    writer.write_results(&matches)?;

    eprintln!(
        "Found {} relevant files for \"{}\" → {}/",
        matches.len(),
        query,
        out_dir.display()
    );

    Ok(())
}

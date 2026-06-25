use crate::cli::PatternsArgs;
use anyhow::Result;

pub fn run(args: PatternsArgs) -> Result<()> {
    eprintln!("patterns: category={:?}", args.category);
    // Placeholder
    Ok(())
}

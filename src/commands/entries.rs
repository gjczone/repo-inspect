use crate::cli::EntriesArgs;
use anyhow::Result;

pub fn run(args: EntriesArgs) -> Result<()> {
    eprintln!("entries: kind={}", args.kind);
    // Placeholder
    Ok(())
}

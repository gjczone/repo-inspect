use crate::cli::DataArgs;
use anyhow::Result;

pub fn run(args: DataArgs) -> Result<()> {
    eprintln!("data: name={:?}", args.name);
    // Placeholder
    Ok(())
}

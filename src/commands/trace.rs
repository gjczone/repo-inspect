use crate::cli::TraceArgs;
use anyhow::Result;

pub fn run(args: TraceArgs) -> Result<()> {
    eprintln!("trace: {} (direction: {:?})", args.symbol, args.direction);
    // Placeholder
    Ok(())
}

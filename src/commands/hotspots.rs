use crate::cli::HotspotsArgs;
use anyhow::Result;

pub fn run(args: HotspotsArgs) -> Result<()> {
    eprintln!("hotspots: count={}", args.count);
    // Placeholder
    Ok(())
}

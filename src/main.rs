mod cli;
mod commands;
mod git;
mod output;
mod search;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let args = cli::Args::parse();
    let repo = args.repo;
    let out_dir = args.out_dir;
    let format = args.output;

    match args.command {
        cli::Command::FindHow(cmd) => commands::find_how::run(cmd, &repo, &out_dir, format),
        cli::Command::Trace(cmd) => commands::trace::run(cmd),
        cli::Command::Entries(cmd) => commands::entries::run(cmd),
        cli::Command::Patterns(cmd) => commands::patterns::run(cmd),
        cli::Command::Data(cmd) => commands::data::run(cmd),
        cli::Command::Hotspots(cmd) => commands::hotspots::run(cmd),
    }
}

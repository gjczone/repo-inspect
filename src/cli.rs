use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Surgical codebase inspection for AI agents.
///
/// repo-inspect helps AI agents quickly understand how specific features
/// are implemented in any codebase. Instead of dumping the entire repo,
/// it surgically extracts relevant code, traces call chains, and detects
/// patterns — producing compact, structured output.
#[derive(Parser)]
#[command(name = "repo-inspect", version, about)]
pub struct Args {
    /// Path to the repository to inspect
    #[arg(short, long, default_value = ".")]
    pub repo: PathBuf,

    /// Output format: json (for agent consumption) or md (for human reading)
    #[arg(short, long, default_value = "md")]
    pub output: OutputFormat,

    /// Output directory (default: .inspect/)
    #[arg(short = 'd', long, default_value = ".inspect")]
    pub out_dir: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Search how a specific feature or concept is implemented
    FindHow(FindHowArgs),
    /// Trace callers and callees of a symbol
    Trace(TraceArgs),
    /// Find entry points: CLI commands, API routes, event handlers, plugin hooks
    Entries(EntriesArgs),
    /// Detect design patterns, conventions, and idioms
    Patterns(PatternsArgs),
    /// Extract core data structures, schemas, and type definitions
    Data(DataArgs),
    /// Identify hotspots: frequently changed or complex files
    Hotspots(HotspotsArgs),
}

#[derive(clap::Args)]
pub struct FindHowArgs {
    /// What to search for: a concept, feature, or technique (e.g., "middleware", "plugin system")
    pub query: Vec<String>,
    /// Limit result depth (1 = just the core file, 2 = + direct callers, 3 = full chain)
    #[arg(short, long, default_value = "2")]
    pub depth: u8,
}

#[derive(clap::Args)]
pub struct TraceArgs {
    /// Symbol to trace (function name, type name, method)
    pub symbol: String,
    /// Direction: callers (who calls this), callees (what this calls), or both
    #[arg(short, long, default_value = "both")]
    pub direction: TraceDirection,
}

#[derive(clap::Args)]
pub struct EntriesArgs {
    /// Filter by entry type: cli, http, event, plugin, or all
    #[arg(short, long, default_value = "all")]
    pub kind: String,
}

#[derive(clap::Args)]
pub struct PatternsArgs {
    /// Filter by pattern category (e.g., creational, structural, concurrency)
    #[arg(short, long)]
    pub category: Option<String>,
}

#[derive(clap::Args)]
pub struct DataArgs {
    /// Filter to a specific type/module name
    #[arg(short, long)]
    pub name: Option<String>,
}

#[derive(clap::Args)]
pub struct HotspotsArgs {
    /// Number of top hotspots to return
    #[arg(short, long, default_value = "10")]
    pub count: usize,
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum OutputFormat {
    Json,
    Md,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum TraceDirection {
    Callers,
    Callees,
    Both,
}

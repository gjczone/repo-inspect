use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;

/// Repository specification: either a local path or a remote GitHub repo.
#[derive(Debug, Clone)]
pub enum RepoSpec {
    Local(PathBuf),
    Remote { owner: String, repo: String },
}

impl FromStr for RepoSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 远程仓库格式: owner/repo (不含前导 / 或 .，恰好一个斜杠)
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2
            && !parts[0].is_empty()
            && !parts[1].is_empty()
            && !parts[0].starts_with('.')
            && !parts[1].starts_with('.')
            && !s.starts_with('/')
        {
            return Ok(RepoSpec::Remote {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
            });
        }
        // 否则视为本地路径
        Ok(RepoSpec::Local(PathBuf::from(s)))
    }
}

impl std::fmt::Display for RepoSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoSpec::Local(p) => write!(f, "{}", p.display()),
            RepoSpec::Remote { owner, repo } => write!(f, "{owner}/{repo}"),
        }
    }
}

/// Surgical codebase inspection for AI agents.
///
/// repo-inspect helps AI agents quickly understand how specific features
/// are implemented in any codebase. Instead of dumping the entire repo,
/// it surgically extracts relevant code, traces call chains, and detects
/// patterns — producing compact, structured output.
#[derive(Parser)]
#[command(name = "repo-inspect", version, about)]
pub struct Args {
    /// Repository to inspect: local path (e.g., ".") or remote GitHub repo (e.g., "owner/repo")
    #[arg(short, long, default_value = ".")]
    pub repo: RepoSpec,

    /// Output format: json (for agent consumption) or md (for human reading)
    #[arg(short, long, default_value = "md")]
    pub output: OutputFormat,

    /// Output directory (default: .inspect/)
    #[arg(short = 'd', long, default_value = ".inspect")]
    pub out_dir: PathBuf,

    /// Force re-fetch remote repo, bypassing local cache
    #[arg(long)]
    pub refresh: bool,

    /// Force full download of all source files in remote mode (Tier 3, skip progressive scanning)
    #[arg(long)]
    pub full: bool,

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
    /// Single-command project spine: architecture overview, dependencies, module structure
    Overview(OverviewArgs),
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
    /// Trace depth: 1 = direct only, 2 = + indirect (default), 3 = full chain
    #[arg(long, default_value = "2")]
    pub depth: usize,
    /// Maximum entries per direction (callers/callees each)
    #[arg(short, long, default_value = "100")]
    pub limit: usize,
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
    /// Maximum number of type definitions to return
    #[arg(short, long, default_value = "50")]
    pub limit: usize,
}

#[derive(clap::Args)]
pub struct HotspotsArgs {
    /// Number of top hotspots to return
    #[arg(short, long, default_value = "10")]
    pub count: usize,
}

#[derive(clap::Args)]
pub struct OverviewArgs {
    /// Optional keyword to focus overview on specific modules/files
    #[arg(short, long)]
    pub filter: Option<String>,
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

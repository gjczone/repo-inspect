mod cli;
mod commands;
mod git;
#[allow(dead_code)]
mod graph;
mod output;
mod remote;
mod scan;
mod search;

use anyhow::Result;
use clap::Parser;
use cli::RepoSpec;
use std::path::PathBuf;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let args = cli::Args::parse();
    let out_dir = args.out_dir;
    let format = args.output;

    // 解析 repo：本地路径直接使用，远程仓库先下载缓存
    let repo: PathBuf = match &args.repo {
        RepoSpec::Local(path) => {
            if !path.exists() {
                anyhow::bail!(
                    "仓库路径不存在: {}。对于远程仓库，请使用 owner/repo 格式。",
                    path.display()
                );
            }
            path.clone()
        }
        RepoSpec::Remote { owner, repo } => remote::prepare(owner, repo, args.refresh)?,
    };

    match args.command {
        cli::Command::FindHow(cmd) => commands::find_how::run(cmd, &repo, &out_dir, format),
        cli::Command::Trace(cmd) => commands::trace::run(cmd, &repo, &out_dir, format),
        cli::Command::Entries(cmd) => commands::entries::run(cmd, &repo, &out_dir, format),
        cli::Command::Patterns(cmd) => commands::patterns::run(cmd, &repo, &out_dir, format),
        cli::Command::Data(cmd) => commands::data::run(cmd, &repo, &out_dir, format),
        cli::Command::Hotspots(cmd) => commands::hotspots::run(cmd, &repo, &out_dir, format),
    }
}

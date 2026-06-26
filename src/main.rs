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
    let format = args.output;

    // 解析 repo：本地路径直接使用，远程仓库先下载缓存。
    // 三阶段渐进式远程扫描：
    //   Tier 1 (lightweight): overview — 只拉元数据，零源文件下载
    //   Tier 2 (selective): find-how / trace — Search API 定位 + 按需下载
    //   Tier 3 (full): 其他命令 / --full 标志 — 全量下载全部源文件
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
        RepoSpec::Remote { owner, repo } => {
            if args.full {
                // --full 强制全量下载（Tier 3），跳过渐进式扫描
                remote::prepare(owner, repo, args.refresh)?
            } else {
                match &args.command {
                    // Tier 1: 轻量级 — 只拉元数据
                    cli::Command::Overview(_) => {
                        remote::prepare_lightweight(owner, repo, args.refresh)?
                    }
                    // Tier 2: 选择性 — Search API + 按需下载
                    cli::Command::FindHow(cmd) => {
                        let query = cmd.query.join(" ");
                        if !query.is_empty() {
                            remote::prepare_selective(owner, repo, &query, args.refresh)?
                        } else {
                            remote::prepare(owner, repo, args.refresh)?
                        }
                    }
                    cli::Command::Trace(cmd) => {
                        remote::prepare_trace(owner, repo, &cmd.symbol, args.refresh)?
                    }
                    // Tier 3: 全量下载（默认）
                    _ => remote::prepare(owner, repo, args.refresh)?,
                }
            }
        }
    };

    // 默认 .inspect/ 写入目标 repo 目录内，而非当前工作目录。
    // 用户显式指定绝对路径时尊重其选择。
    let out_dir = if args.out_dir.is_relative() {
        repo.join(&args.out_dir)
    } else {
        args.out_dir
    };

    match args.command {
        cli::Command::FindHow(cmd) => commands::find_how::run(cmd, &repo, &out_dir, format),
        cli::Command::Trace(cmd) => commands::trace::run(cmd, &repo, &out_dir, format),
        cli::Command::Entries(cmd) => commands::entries::run(cmd, &repo, &out_dir, format),
        cli::Command::Patterns(cmd) => commands::patterns::run(cmd, &repo, &out_dir, format),
        cli::Command::Data(cmd) => commands::data::run(cmd, &repo, &out_dir, format),
        cli::Command::Hotspots(cmd) => commands::hotspots::run(cmd, &repo, &out_dir, format),
        cli::Command::Overview(cmd) => commands::overview::run(cmd, &repo, &out_dir, format),
    }
}

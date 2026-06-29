# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-06-28

Initial release — surgical codebase inspection CLI for AI agents.

### Features

- **7 subcommands**: `overview`, `find-how`, `trace`, `entries`, `patterns`, `data`, `hotspots`
- **Local mode**: `.gitignore`-aware file walking, zero network
- **Remote mode**: inspect any public GitHub repo without cloning (`--repo owner/repo`)
- **Three-tier progressive remote scanning**: overview (metadata only) → selective (search API) → full download
- **L2 tree-sitter parsing**: structured symbol extraction for Rust, Python, TypeScript, Go
- **Rayon parallel pipeline**: parallel file parsing + parallel remote downloads
- **CompiledQueries caching**: tree-sitter Query objects compiled once per language, reused across all files
- **Dual output**: Markdown (default) and JSON (`--output json`)
- **Skill distribution**: bundled binary under `skills/repo-inspect/scripts/` for `npx skills add`

## [0.1.2] - 2026-06-29

### Fixed (16 issues)

- **远程缓存安全**: `prepare_lightweight()` 不再删除整个缓存目录，仅清理轻量级文件；添加 `CacheMode` 防止轻量级缓存被误用为完整缓存 (#27, #40, #30, #46)
- **路径遍历防护**: 添加 `safe_join` 辅助函数防止路径穿越攻击 CWE-22 (#28, #41)
- **原子写入**: 使用临时文件+重命名机制防止缓存损坏 (#31, #42)
- **下载错误率阈值**: 并行下载错误率 >10% 时报错而非静默产出不完整结果 (#32, #49)
- **时钟异常处理**: `now_secs()` 在时钟异常时打印警告并返回 0 强制刷新，`check_cache()` 使用 `checked_sub` 检测时钟倒退 (#33, #48, #37, #52)
- **速率限制检测**: 改为 JSON 解析而非脆弱字符串匹配，错误消息安全截断 (#34, #51)
- **404 回退优化**: `fetch_raw_file()` 仅在 404 时回退到 Contents API，网络错误直接传播 (#35, #50)
- **元数据损坏恢复**: `ensure_cached()` 在元数据损坏时重建而非静默替换为空默认值 (#36)
- **NaN 安全排序**: search 排序使用 `unwrap_or(Ordering::Equal)` 防止 NaN panic (#38, #44)
- **UTF-8 安全截断**: `sanitize_filename` 使用 `chars().take()` 防止多字节边界 panic (#39)
- **错误传播**: `FileFinder::search()` 返回 `Result` 而非静默返回空结果 (#45)
- **远程 --out-dir 修复**: 远程模式下 `--out-dir` 解析到当前工作目录而非缓存目录深处 (#47)
- **unwrap 安全替换**: `graph/builder.rs` 中多个 `unwrap()` 替换为安全模式 (#53)
- **新增源文件扩展名**: 添加 `r`, `jl`, `ex`, `exs`, `erl`, `hrl`, `dart` (#29, #43)

### Changed

- `find-how` 和 `FileFinder::search()` 返回 `Result`，调用方需处理可能的遍历错误
- 远程模式下 `--out-dir` 的相对路径解析到 `cwd` 而非缓存目录

## [Unreleased]

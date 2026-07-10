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

## [0.1.4] - 2026-07-10

### Fixed

- **trace 命令空切片守卫 (#57)**: `write_trace_markdown` 内部增加 `if symbols.is_empty() { return Ok(()); }` 防御性守卫，消除跨函数 `symbols[0]` 索引的 panic 风险（REVIEW-RULES P0 #1 guard-then-index 模式）。非破坏性变更。

### Review 遗留 (已开 issue，非阻塞)

- #77 `check_cache` 对 Full 模式也需校验关键缓存文件存在性
- #78 PageRank 在 `max_iter=0` 时显式归一或早返回（防御性边界）

## [0.1.3] - 2026-07-10

### Fixed

- **远程模块健壮性 (#56, #60, #61, #62, #63, #64)**:
  - `truncate_body`/`sanitize_url` 改用 `chars().take()` 做 Unicode 安全截断，修复多字节 UTF-8 边界 panic (#56)
  - `check_cache` 轻量级模式增加 `tree.json` 存在性校验，避免刷新中断后读到半截缓存 (#60)
  - 删除死代码 `ensure_cached` (#61)
  - HTTP 请求增加 30s 超时 (`with_timeout`)，避免无限挂起 (#62)
  - `read_cache_meta` 错误与 `remove_file` 失败改为 `debug!` 日志，不再静默吞错 (#63, #64)
  - `src/remote/search.rs` 删除重复的 `truncate_body`/`sanitize_url`，复用 `mod.rs` 定义
- **扫描/搜索模块排序可复现 (#58, #59, #67)**:
  - `find_symbols` 改为三级匹配质量（精确/前缀/包含）+ 稳定 tiebreaker `(file, line, name)`，排序结果完全确定 (#59)
  - `find_call_refs` 按 `(file, line)` 稳定排序 (#58)
  - 统一 `is_source_file_name`，本地/远程共用一份扩展名判定，消除复制分叉风险 (#67)
- **图模块算法正确性 (#66)**:
  - PageRank 改用 `pr_a`/`pr_b` 双缓冲 + `swap`，避免每轮重分配；预计算 `dangling_indices`
  - `traverse` BFS 直接遍历边列表，去除每轮 `collect` 临时 Vec
- **输出/命令模块数据修正 (#68)**:
  - `OutputWriter` 新增 `query` 字段，标题/JSON 使用原始查询而非文件名 stem（修复 `find-how-` 前缀错误）
  - `parse_cargo_dep_line` 对 git 依赖无版本返回空串，依赖表空版本显示 `-`，移除未用 `file_symbol_counts`

### Security

- **依赖升级**: `crossbeam-epoch` 0.9.18 → 0.9.20，修复 `RUSTSEC-2026-0204` 漏洞

### Review 遗留 (已开 issue，非阻塞)

- #77 `check_cache` 对 Full 模式也需校验关键缓存文件存在性
- #78 PageRank 在 `max_iter=0` 时显式归一或早返回（防御性边界）

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

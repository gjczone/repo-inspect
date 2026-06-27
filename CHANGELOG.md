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

## [Unreleased]

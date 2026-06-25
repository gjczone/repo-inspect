# repo-inspect L2+L3 Design

## Vision

> `repo-inspect` 应该让 LLM 像你研究 pi-subagents/kimi-code/pi-crew 来构建 pi-swarm 一样，快速、精准地理解任何开源项目的实现逻辑。

L1 (current): 尊重 `.gitignore` 的智能 grep —— 关键词搜索 + 上下文
L2 (target): Tree-sitter 语言感知 —— 按语法结构提取符号、区分定义/调用/引用
L3 (target): 调用图 + 依赖图 —— 追踪调用链、计算重要性、检测模式

---

## What L2+L3 Enables (Per Command)

### `find-how "query"` — L2 增强

| 维度 | L1 (now) | L2 (after) |
|------|---------|-----------|
| 搜索 | 文本匹配 `to_lowercase().contains()` | 按符号类型搜索：只匹配函数/类型/接口定义，跳过注释和字符串 |
| 精确度 | 搜"error"命中注释 `// handle error` | 搜"error"命中 `class AppError` / `fn handle_error()` 定义 |
| 发现 | 不知道搜什么 | `--discover` 模式：自动提取所有符号名，按 PageRank 排序，让 LLM 发现"这个项目里最重要的是什么" |

### `trace <symbol>` — L3 全新能力

```
trace "applyMiddleware"
  ↓
找到定义 → 调用者(callers) → 被调用者(callees) → 递归到指定深度
  ↓
输出:
  applyMiddleware (src/applyMiddleware.ts:12)
    ← called by: createStore (src/createStore.ts:89)
    → calls: compose (src/compose.ts:22)
    → access: store.dispatch, store.getState
```

### `entries` — L2 增强

| L1 (stub) | L2 |
|-----------|-----|
| 空壳 | 检测 CLI 入口 `fn main()` / `def main()` / `if __name__` / bin 字段 |
| | 检测 HTTP 路由：`@Get("/api/users")` / `app.get("/api/users")` / Flask route decorators |
| | 检测事件/插件注册：`registry.register()` / `plugin.load()` 模式 |
| | 按语言自动选择检测策略 |

### `patterns` — L3 全新能力

```
patterns --category concurrency
  ↓ 图分析
检测:
  - async/await 热点: 5 files, 12 async functions
  - 锁模式: Mutex<HashMap<...>> (src/cache.rs:34)
  - 通道: mpsc::channel (src/worker.rs:89)
  - 线程池: rayon::ThreadPool (src/main.rs:12)
```

### `data` — L2 增强

| L1 | L2 |
|-----|-----|
| 文本搜索 "struct" / "interface" | tree-sitter 提取 struct/enum/interface/type 定义 + 字段 + 方法 |
| | 检测数据流：函数参数 → 返回值 → 持久化（类型追踪） |

### `hotspots` — L3 增强

| L1 | L3 |
|-----|-----|
| 文件大小 + git churn | PageRank 重要性排序 |
| | 圈复杂度（AST 节点计数） |
| | 耦合度（入边+出边数） |
| | 综合评分：churn × 0.4 + pagerank × 0.3 + complexity × 0.3 |

---

## Architecture (adapted from pi-shazam)

```
repo-inspect (Rust)
├── src/
│   ├── main.rs              # CLI entry
│   ├── cli.rs               # Clap args
│   ├── commands/            # Subcommands (L2/L3 aware)
│   │   ├── find_how.rs
│   │   ├── trace.rs
│   │   ├── entries.rs
│   │   ├── patterns.rs
│   │   ├── data.rs
│   │   └── hotspots.rs
│   ├── scan/                # ── NEW: L2 core ──
│   │   ├── mod.rs           #   3-phase pipeline coordinator
│   │   ├── walker.rs        #   File discovery (port from search/mod.rs)
│   │   ├── parser.rs        #   Tree-sitter adapter + language dispatch
│   │   ├── queries.rs       #   S-expression patterns per language
│   │   └── resolver.rs      #   Import/module resolution per language
│   ├── graph/               # ── NEW: L3 core ──
│   │   ├── mod.rs           #   Symbol graph data structures
│   │   ├── builder.rs       #   Edge construction (import/call/ref)
│   │   ├── pagerank.rs      #   Pure iterative PageRank
│   │   └── traverse.rs      #   BFS/DFS call chain traversal
│   ├── search/              # L1 (keep, used as fallback + quick mode)
│   │   └── mod.rs
│   └── output/              # Output formatting
│       └── mod.rs
```

### 3-Phase Scan Pipeline (from pi-shazam)

```
Phase 1: Parse all files
  ├── walk files (ignore crate, .gitignore-aware)
  ├── detect language per file (.rs → Rust, .ts → TypeScript, etc.)
  ├── tree-sitter parse → extract: symbols, imports, calls, refs
  └── store in flat vec (no graph yet)

Phase 2: Build edges
  ├── resolve imports per language (Rust crate::, TS ./, Python dotted)
  ├── create import edges: importing-file → imported-file symbols (weight=0.3)
  ├── create call edges: caller symbol → callee symbol (weight=1.0, name-match)
  └── create ref edges: same-file identifier references (weight=0.5)

Phase 3: Score
  └── PageRank on edge-weighted graph (d=0.85, 50 iter, ε=1e-6)
```

### Graph Data Model (simplified from pi-shazam)

```rust
struct SymbolGraph {
    symbols: HashMap<SymbolId, Symbol>,
    name_index: HashMap<String, Vec<SymbolId>>,    // O(1) lookup
    outgoing: HashMap<SymbolId, Vec<Edge>>,
    incoming: HashMap<SymbolId, Vec<Edge>>,
    file_symbols: HashMap<PathBuf, Vec<SymbolId>>,  // file → its symbols
    file_imports: HashMap<PathBuf, Vec<PathBuf>>,    // file → imported files
}

struct Symbol {
    id: SymbolId,           // "src/cli.rs::Args::15"
    name: String,           // "Args"
    kind: SymbolKind,       // Function, Struct, Interface, Enum, Method, Module
    file: PathBuf,
    line: usize,
    pagerank: f64,
    signature: String,      // extracted from tree-sitter node text
}

enum SymbolKind { Function, Method, Struct, Enum, Interface, Trait, 
                  TypeAlias, Module, Variable, Const }

struct Edge {
    source: SymbolId,
    target: SymbolId,
    kind: EdgeKind,
    weight: f64,
}

enum EdgeKind { Import, Call, Ref }
```

### Tree-Sitter Language Matrix

| Language | Rust Crate | Extensions | Key Query Types |
|----------|-----------|------------|-----------------|
| Rust | `tree-sitter-rust` | `.rs` | function, struct, enum, trait, impl, mod, use, call |
| Python | `tree-sitter-python` | `.py`, `.pyi` | function, class, import, call, decorator |
| TypeScript | `tree-sitter-typescript` | `.ts`, `.tsx` | function, class, interface, type, import, call, jsx |
| Go | `tree-sitter-go` | `.go` | function, method, struct, interface, import, call |

### S-Expression Query Porting

pi-shazam's queries are in TypeScript string literals. We port them to Rust `&str` constants:

```rust
// queries.rs
pub const PYTHON_FUNCTION_QUERY: &str = r#"
(function_definition name: (identifier) @name) @definition.function
(decorated_definition (function_definition name: (identifier) @name)) @definition.function
(class_definition body: (block (function_definition name: (identifier) @name))) @definition.method
"#;

pub const TYPESCRIPT_IMPORT_QUERY: &str = r#"
(import_statement source: (string) @source)
(import_specifier name: (identifier) @name)
"#;
```

Exact same S-expression syntax — tree-sitter query language is universal.

### Import Resolution Per Language

| Language | Pattern | Resolution |
|----------|---------|-----------|
| Rust | `use crate::foo::bar` | `crate::` → src root; `super::` → parent; `mod foo;` → `foo.rs` or `foo/mod.rs` |
| TypeScript | `import { x } from './foo'` | Extensionless → try `.ts`, `.tsx`, `.js`, `.jsx`, `/index.ts`, etc. |
| Python | `from foo.bar import Baz` | `foo.bar` → `foo/bar.py` or `foo/bar/__init__.py` |
| Go | `import "github.com/x/y"` | External — skip. Local `"./foo"` → `foo.go` |

### PageRank in Rust

```rust
fn pagerank(graph: &SymbolGraph, damping: f64, max_iter: usize, tol: f64) -> HashMap<SymbolId, f64> {
    let n = graph.symbols.len();
    let mut scores: HashMap<_, f64> = symbols.iter().map(|(id, _)| (*id, 1.0 / n as f64)).collect();
    
    for _ in 0..max_iter {
        let mut new_scores = HashMap::new();
        let dangling_sum: f64 = /* sum of scores for nodes with no outgoing edges */;
        
        for (id, symbol) in &graph.symbols {
            let incoming_score: f64 = graph.incoming.get(id)
                .map(|edges| edges.iter().map(|e| scores[&e.source] * e.weight).sum())
                .unwrap_or(0.0);
            new_scores.insert(*id, (1.0 - damping) / n as f64 + damping * (incoming_score + dangling_sum / n as f64));
        }
        
        // Check convergence
        if max_diff(&scores, &new_scores) < tol { break; }
        scores = new_scores;
    }
    scores
}
```

---

## Implementation Plan

### Phase A: Tree-sitter Integration (L2 core)

1. Add `tree-sitter`, `tree-sitter-python`, `tree-sitter-typescript`, `tree-sitter-rust`, `tree-sitter-go` to `Cargo.toml`
2. Create `src/scan/queries.rs` — port all S-expression patterns from pi-shazam
3. Create `src/scan/parser.rs` — `TreeSitterAdapter`:
   - `detect_language(path) -> Option<Language>`
   - `parse_file(path) -> ParseResult { symbols, imports, calls, refs }`
   - Uses precompiled queries per language
4. Create `src/scan/walker.rs` — renamed from `search/mod.rs`, keep existing `FileFinder`
5. Update `find_how` to use tree-sitter symbol filtering

### Phase B: Graph Engine (L3 core)

6. Create `src/graph/mod.rs` — `SymbolGraph` data structures
7. Create `src/graph/builder.rs` — 3-phase builder
8. Create `src/graph/pagerank.rs`
9. Create `src/graph/traverse.rs` — call chain BFS

### Phase C: Command Upgrade

10. `trace` — full implementation with caller/callee traversal
11. `entries` — language-aware entry point detection
12. `patterns` — graph-pattern detection (singleton, factory, observer, etc.)
13. `data` — tree-sitter struct/type extraction
14. `hotspots` — PageRank + complexity scoring

### Phase D: Full-Stack Detection

15. Multi-language project detection: find package.json + Cargo.toml + go.mod + pyproject.toml
16. Cross-boundary awareness: TS frontend calling Python/Rust backend → flag it
17. Technology stack summary in output

---

## Performance Budget

| Metric | Target |
|--------|--------|
| Parse 10k LOC (single file) | < 50ms |
| Parse 100k LOC (project) | < 3s |
| Graph build (100k LOC) | < 5s |
| PageRank (10k symbols) | < 500ms |
| Total: cold scan 100k LOC | < 10s |
| Binary size (release) | < 8 MB (with 4 tree-sitter grammars) |

---

## Dependencies to Add

```toml
[dependencies]
# Tree-sitter (L2)
tree-sitter = "0.24"
tree-sitter-python = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-rust = "0.23"
tree-sitter-go = "0.23"
```

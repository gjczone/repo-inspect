//! Tree-sitter adapter: 语言检测 + 文件解析 + 符号提取。
//!
//! 职责:
//! - 根据文件扩展名检测语言
//! - 用 tree-sitter 解析源码，提取符号/导入/调用
//! - 输出结构化的 `ParsedFile` 供上层使用

use std::path::Path;

/// 支持的编程语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    Go,
}

/// 符号类别 — 对应 tree-sitter 查询中的 capture 名称。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Interface,
    TypeAlias,
    Class,
    Module,
    Impl,
}

impl SymbolKind {
    /// 从 tree-sitter capture 名称转换（如 "definition.function" → Function）。
    pub fn from_capture(capture: &str) -> Option<Self> {
        match capture {
            "definition.function" => Some(Self::Function),
            "definition.method" => Some(Self::Method),
            "definition.struct" => Some(Self::Struct),
            "definition.enum" => Some(Self::Enum),
            "definition.trait" => Some(Self::Trait),
            "definition.interface" => Some(Self::Interface),
            "definition.type_alias" => Some(Self::TypeAlias),
            "definition.type" => Some(Self::TypeAlias),
            "definition.class" => Some(Self::Class),
            "definition.module" => Some(Self::Module),
            "definition.impl" => Some(Self::Impl),
            "definition.trait_method" => Some(Self::Method),
            _ => None,
        }
    }

    /// 用于 Markdown 输出的标签。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::TypeAlias => "type",
            Self::Class => "class",
            Self::Module => "mod",
            Self::Impl => "impl",
        }
    }
}

/// 提取到的符号定义。
#[derive(Debug, Clone)]
pub struct ExtractedSymbol {
    /// 符号名称
    pub name: String,
    /// 符号类别
    pub kind: SymbolKind,
    /// 起始行（1-based）
    pub line: usize,
    /// 结束行（1-based）
    pub end_line: usize,
    /// 签名/第一行文本（截断到合理长度）
    pub signature: String,
}

/// 提取到的导入声明。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ImportDecl {
    /// 模块路径（如 "crate::scan::parser"、"./utils"、"os.path"）
    pub module: String,
    /// 行号（1-based）
    pub line: usize,
}

/// 提取到的函数调用引用。
#[derive(Debug, Clone)]
pub struct CallRef {
    /// 被调用的函数/方法名
    pub name: String,
    /// 行号（1-based）
    pub line: usize,
}

/// 单个文件的解析结果。
#[derive(Debug, Clone)]
pub struct ParsedFile {
    /// 文件路径（相对于 repo root）
    pub path: std::path::PathBuf,
    /// 提取到的符号定义
    pub symbols: Vec<ExtractedSymbol>,
    /// 提取到的导入声明
    pub imports: Vec<ImportDecl>,
    /// 提取到的函数调用
    pub calls: Vec<CallRef>,
}

/// 根据文件扩展名检测语言。
///
/// 返回 `None` 表示不支持该文件类型（非源码文件或未覆盖的语言）。
pub fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "rs" => Some(Language::Rust),
        "py" | "pyi" => Some(Language::Python),
        "ts" | "mts" | "cts" => Some(Language::TypeScript),
        "tsx" => Some(Language::TypeScript), // TSX 共用 TypeScript 查询
        "go" => Some(Language::Go),
        _ => None,
    }
}

/// 获取语言对应的 tree-sitter `Language` 对象。
///
/// 用于 `Parser::set_language()` 和 `Query::new()`。
pub fn ts_language(lang: Language) -> tree_sitter::Language {
    match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
    }
}

/// 获取语言对应的查询模式。
pub fn queries_for(lang: Language) -> crate::scan::queries::LanguageQueries {
    match lang {
        Language::Rust => crate::scan::queries::rust_queries(),
        Language::Python => crate::scan::queries::python_queries(),
        Language::TypeScript => crate::scan::queries::typescript_queries(),
        Language::Go => crate::scan::queries::go_queries(),
    }
}

/// 预编译的 tree-sitter 查询对象。
///
/// 同一语言的所有文件共享同一组编译后的 Query，避免每个文件重复编译。
/// 4 种语言 × 3 种查询 = 最多 12 个 Query 对象，而非每个文件 3 次编译。
pub struct CompiledQueries {
    pub symbols: tree_sitter::Query,
    pub imports: tree_sitter::Query,
    pub calls: tree_sitter::Query,
}

/// 按语言预编译 tree-sitter 查询。
///
/// 返回 `None` 当查询编译失败时（通常意味着查询模式有语法错误）。
pub fn compile_queries(lang: Language) -> Option<CompiledQueries> {
    let ts_lang = ts_language(lang);
    let queries = queries_for(lang);
    Some(CompiledQueries {
        symbols: tree_sitter::Query::new(&ts_lang, queries.symbols).ok()?,
        imports: tree_sitter::Query::new(&ts_lang, queries.imports).ok()?,
        calls: tree_sitter::Query::new(&ts_lang, queries.calls).ok()?,
    })
}

/// 用 tree-sitter 解析源文件，提取符号/导入/调用。
///
/// 返回 `None` 当语言不支持或解析失败时。
/// 便捷包装：内部编译查询后调用 `parse_file_with_queries`。
/// 批量解析场景应使用 `parse_file_with_queries` + 预编译查询。
#[allow(dead_code)]
pub fn parse_file(path: &Path, source: &[u8]) -> Option<ParsedFile> {
    let lang = detect_language(path)?;
    let queries = compile_queries(lang)?;
    parse_file_with_queries(path, source, lang, &queries)
}

/// 用预编译的查询解析源文件。
///
/// 与 `parse_file` 功能相同，但接受预编译的 `CompiledQueries`，避免重复编译。
/// `scan_project` 应优先使用此函数以获得更好的性能。
pub fn parse_file_with_queries(
    path: &Path,
    source: &[u8],
    lang: Language,
    queries: &CompiledQueries,
) -> Option<ParsedFile> {
    use tree_sitter::{Parser, QueryCursor, StreamingIterator};

    let ts_lang = ts_language(lang);

    let mut parser = Parser::new();
    parser.set_language(&ts_lang).ok()?;
    let tree = parser.parse(source, None)?;

    let root = tree.root_node();

    let mut symbols = Vec::new();
    let mut imports = Vec::new();
    let mut calls = Vec::new();

    // 提取符号定义
    {
        let query = &queries.symbols;
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, root, source);
        while let Some(m) = matches.next() {
            let mut name_text = None;
            let mut def_node = None;
            let mut capture_name = None;
            for ci in 0..m.captures.len() {
                let cap = &m.captures[ci];
                let cname = query.capture_names()[cap.index as usize];
                if cname == "name" {
                    name_text = std::str::from_utf8(&source[cap.node.byte_range()]).ok();
                } else {
                    capture_name = Some(cname);
                    def_node = Some(cap.node);
                }
            }
            if let (Some(name), Some(node), Some(capture)) = (name_text, def_node, capture_name)
                && let Some(kind) = SymbolKind::from_capture(capture)
            {
                // 提取签名：取定义节点的第一行，截断到 200 字符
                let node_text = std::str::from_utf8(&source[node.byte_range()]).unwrap_or("");
                let signature = node_text
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(200)
                    .collect::<String>();

                symbols.push(ExtractedSymbol {
                    name: name.to_string(),
                    kind,
                    line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    signature,
                });
            }
        }
    }

    // 提取导入声明
    {
        let query = &queries.imports;
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, root, source);
        while let Some(m) = matches.next() {
            let mut module_text = None;
            let mut line = 0;
            for ci in 0..m.captures.len() {
                let cap = &m.captures[ci];
                let cname = query.capture_names()[cap.index as usize];
                // @name, @source, @path 都是导入目标
                if cname == "name" || cname == "source" || cname == "path" {
                    module_text = std::str::from_utf8(&source[cap.node.byte_range()]).ok();
                    line = cap.node.start_position().row + 1;
                }
            }
            if let Some(module) = module_text {
                // 去掉字符串引号
                let module = module.trim_matches('"').trim_matches('\'').to_string();
                if !module.is_empty() {
                    imports.push(ImportDecl { module, line });
                }
            }
        }
    }

    // 提取函数调用
    {
        let query = &queries.calls;
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, root, source);
        while let Some(m) = matches.next() {
            let mut call_name = None;
            let mut line = 0;
            for ci in 0..m.captures.len() {
                let cap = &m.captures[ci];
                let cname = query.capture_names()[cap.index as usize];
                if cname == "name" {
                    call_name = std::str::from_utf8(&source[cap.node.byte_range()]).ok();
                    line = cap.node.start_position().row + 1;
                }
            }
            if let Some(name) = call_name {
                calls.push(CallRef {
                    name: name.to_string(),
                    line,
                });
            }
        }
    }

    Some(ParsedFile {
        path: path.to_path_buf(),
        symbols,
        imports,
        calls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("foo.rs")), Some(Language::Rust));
        assert_eq!(detect_language(Path::new("foo.py")), Some(Language::Python));
        assert_eq!(
            detect_language(Path::new("foo.pyi")),
            Some(Language::Python)
        );
        assert_eq!(
            detect_language(Path::new("foo.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            detect_language(Path::new("foo.tsx")),
            Some(Language::TypeScript)
        );
        assert_eq!(detect_language(Path::new("foo.go")), Some(Language::Go));
        assert_eq!(detect_language(Path::new("foo.txt")), None);
        assert_eq!(detect_language(Path::new("foo")), None);
    }

    #[test]
    fn test_parse_rust_struct() {
        let source = b"struct Foo {\n    x: i32,\n    y: String,\n}\n";
        let result = parse_file(Path::new("test.rs"), source);
        let parsed = result.expect("should parse Rust file");
        assert_eq!(parsed.symbols.len(), 1);
        assert_eq!(parsed.symbols[0].name, "Foo");
        assert_eq!(parsed.symbols[0].kind, SymbolKind::Struct);
        assert_eq!(parsed.symbols[0].line, 1);
    }

    #[test]
    fn test_parse_rust_function() {
        let source = b"fn bar(x: i32) -> bool {\n    x > 0\n}\n";
        let result = parse_file(Path::new("test.rs"), source);
        let parsed = result.expect("should parse Rust file");
        assert_eq!(parsed.symbols.len(), 1);
        assert_eq!(parsed.symbols[0].name, "bar");
        assert_eq!(parsed.symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_parse_rust_enum_and_trait() {
        let source = b"enum Color { Red, Green, Blue }\ntrait Drawable {\n    fn draw(&self);\n}\n";
        let result = parse_file(Path::new("test.rs"), source);
        let parsed = result.expect("should parse");
        let names: Vec<&str> = parsed.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Color"), "should find enum Color");
        assert!(names.contains(&"Drawable"), "should find trait Drawable");
    }

    #[test]
    fn test_parse_rust_use() {
        let source = b"use crate::scan::parser;\nuse std::path::Path;\n";
        let result = parse_file(Path::new("test.rs"), source);
        let parsed = result.expect("should parse");
        assert!(parsed.imports.len() >= 2, "should find 2+ imports");
        let modules: Vec<&str> = parsed.imports.iter().map(|i| i.module.as_str()).collect();
        assert!(
            modules.iter().any(|m| m.contains("scan")),
            "should find scan import"
        );
    }

    #[test]
    fn test_parse_rust_call() {
        let source = b"fn main() {\n    foo();\n    bar::baz();\n}\n";
        let result = parse_file(Path::new("test.rs"), source);
        let parsed = result.expect("should parse");
        let call_names: Vec<&str> = parsed.calls.iter().map(|c| c.name.as_str()).collect();
        assert!(call_names.contains(&"foo"), "should find foo() call");
    }

    #[test]
    fn test_distinguish_definition_vs_reference() {
        let source = b"struct Args {\n    name: String,\n}\n\nfn main() {\n    let a = Args { name: \"test\".into() };\n    println!(\"{:?}\", a);\n}\n";
        let result = parse_file(Path::new("test.rs"), source);
        let parsed = result.expect("should parse");

        // Args 应该作为 struct 定义出现
        let args_def = parsed.symbols.iter().find(|s| s.name == "Args");
        assert!(args_def.is_some(), "should find Args as definition");
        assert_eq!(args_def.unwrap().kind, SymbolKind::Struct);

        // main 应该作为 function 定义出现
        let main_def = parsed.symbols.iter().find(|s| s.name == "main");
        assert!(main_def.is_some(), "should find main as definition");
        assert_eq!(main_def.unwrap().kind, SymbolKind::Function);
    }

    #[test]
    fn test_parse_python_function_and_class() {
        let source = b"class Foo:\n    def bar(self):\n        pass\n\ndef baz():\n    pass\n";
        let result = parse_file(Path::new("test.py"), source);
        let parsed = result.expect("should parse Python");
        let names: Vec<&str> = parsed.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Foo"), "should find class Foo");
        assert!(names.contains(&"baz"), "should find function baz");
    }

    #[test]
    fn test_ts_language_not_panics() {
        // 确保所有语言的 Language 对象都能创建
        let _ = ts_language(Language::Rust);
        let _ = ts_language(Language::Python);
        let _ = ts_language(Language::TypeScript);
        let _ = ts_language(Language::Go);
    }
}

//! S-expression query patterns for tree-sitter, ported from pi-shazam.
//!
//! 每种语言定义 3 类查询: symbols（定义）、imports（导入）、calls（调用）。
//! 查询语法是 tree-sitter 标准 S-expression，跨语言通用。

/// 一组语言的 tree-sitter 查询模式。
/// 字段为 `Option`，因为某些语言可能不支持特定查询类别。
pub struct LanguageQueries {
    /// 符号定义: function, struct, class, enum, trait, interface, impl, type, mod
    pub symbols: &'static str,
    /// 导入声明: use, import, from...import, extern crate
    pub imports: &'static str,
    /// 函数/方法调用
    pub calls: &'static str,
}

/// Rust 语言查询。
///
/// 匹配: function_item, struct_item, enum_item, trait_item, impl_item,
///        type_item, mod_item, use_declaration, call_expression
pub fn rust_queries() -> LanguageQueries {
    LanguageQueries {
        symbols: r#"
(function_item name: (identifier) @name) @definition.function
(function_signature_item name: (identifier) @name) @definition.trait_method
(struct_item name: (type_identifier) @name) @definition.struct
(enum_item name: (type_identifier) @name) @definition.enum
(trait_item name: (type_identifier) @name) @definition.trait
(impl_item type: (type_identifier) @name) @definition.impl
(type_item name: (type_identifier) @name) @definition.type
(mod_item name: (identifier) @name) @definition.module
"#,
        imports: r#"
(use_declaration argument: (scoped_identifier) @name)
(use_declaration argument: (identifier) @name)
(extern_crate_declaration name: (identifier) @name)
(mod_item name: (identifier) @name)
"#,
        calls: r#"
(call_expression function: (identifier) @name) @reference.call
(call_expression function: (field_expression field: (field_identifier) @name)) @reference.call
(call_expression function: (scoped_identifier name: (identifier) @name)) @reference.call
"#,
    }
}

/// Python 语言查询。
///
/// 匹配: function_definition, class_definition, import_statement, import_from_statement, call
pub fn python_queries() -> LanguageQueries {
    LanguageQueries {
        symbols: r#"
(function_definition name: (identifier) @name) @definition.function
(decorated_definition (function_definition name: (identifier) @name)) @definition.function
(class_definition body: (block (function_definition name: (identifier) @name))) @definition.method
(class_definition name: (identifier) @name) @definition.class
(decorated_definition (class_definition name: (identifier) @name)) @definition.class
"#,
        imports: r#"
(import_statement name: (dotted_name) @name)
(import_statement name: (aliased_import name: (dotted_name) @name))
(import_from_statement module_name: (dotted_name) @name)
(import_from_statement module_name: (relative_import) @name)
"#,
        calls: r#"
(call function: (identifier) @name) @reference.call
(call function: (attribute attribute: (identifier) @name)) @reference.call
"#,
    }
}

/// TypeScript 语言查询。
///
/// 匹配: function_declaration, variable_declarator(arrow_function), method_definition,
///        class_declaration, interface_declaration, type_alias_declaration, enum_declaration,
///        import_statement, call_expression
pub fn typescript_queries() -> LanguageQueries {
    LanguageQueries {
        symbols: r#"
(function_declaration name: (identifier) @name) @definition.function
(variable_declarator name: (identifier) @name value: (arrow_function)) @definition.function
(method_definition name: (property_identifier) @name) @definition.method
(class_declaration name: (_) @name) @definition.class
(interface_declaration name: (type_identifier) @name) @definition.interface
(type_alias_declaration name: (type_identifier) @name) @definition.type_alias
(enum_declaration name: (identifier) @name) @definition.enum
"#,
        imports: r#"
(import_statement source: (string) @source)
(import_specifier name: (identifier) @name)
(import_clause (identifier) @name)
"#,
        calls: r#"
(call_expression function: (identifier) @name) @reference.call
(call_expression function: (member_expression property: (property_identifier) @name)) @reference.call
"#,
    }
}

/// Go 语言查询。
///
/// 匹配: function_declaration, method_declaration, type_spec(struct/interface),
///        import_spec, call_expression
pub fn go_queries() -> LanguageQueries {
    LanguageQueries {
        symbols: r#"
(function_declaration name: (identifier) @name) @definition.function
(method_declaration name: (field_identifier) @name) @definition.method
(type_spec name: (type_identifier) @name type: (struct_type)) @definition.struct
(type_spec name: (type_identifier) @name type: (interface_type)) @definition.interface
"#,
        imports: r#"
(import_spec path: (interpreted_string_literal) @path)
"#,
        calls: r#"
(call_expression function: (identifier) @name) @reference.call
(call_expression function: (selector_expression field: (field_identifier) @name)) @reference.call
"#,
    }
}

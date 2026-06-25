//! 项目技术栈检测。
//!
//! 扫描项目根目录的配置文件，识别语言、框架和包管理器。

use std::path::Path;

/// 项目技术栈信息。
#[derive(Debug, Clone, Default)]
pub struct ProjectStack {
    /// 检测到的编程语言
    pub languages: Vec<String>,
    /// 检测到的框架/库
    pub frameworks: Vec<String>,
    /// 检测到的包管理器
    pub package_managers: Vec<String>,
}

impl ProjectStack {
    /// 是否为空（未检测到任何技术栈信息）。
    pub fn is_empty(&self) -> bool {
        self.languages.is_empty()
    }

    /// 格式化为单行摘要。
    pub fn summary(&self) -> String {
        let langs = if self.languages.is_empty() {
            "Unknown".to_string()
        } else {
            self.languages.join(", ")
        };
        let pm = if self.package_managers.is_empty() {
            String::new()
        } else {
            format!(" ({})", self.package_managers.join(", "))
        };
        format!("{}{}", langs, pm)
    }
}

/// 检测项目技术栈。
///
/// 扫描根目录下的配置文件来推断语言和框架。
pub fn detect_stack(root: &Path) -> ProjectStack {
    let mut stack = ProjectStack::default();

    // Rust
    if root.join("Cargo.toml").exists() {
        stack.languages.push("Rust".to_string());
        stack.package_managers.push("cargo".to_string());
        extract_rust_frameworks(root, &mut stack);
    }

    // TypeScript / JavaScript
    if root.join("package.json").exists() {
        if root.join("tsconfig.json").exists() {
            stack.languages.push("TypeScript".to_string());
        } else {
            stack.languages.push("JavaScript".to_string());
        }
        detect_node_package_manager(root, &mut stack);
        extract_node_frameworks(root, &mut stack);
    }

    // Go
    if root.join("go.mod").exists() {
        stack.languages.push("Go".to_string());
        stack.package_managers.push("go modules".to_string());
    }

    // Python
    if root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("requirements.txt").exists()
    {
        stack.languages.push("Python".to_string());
        if root.join("pyproject.toml").exists() {
            stack.package_managers.push("pip/pyproject".to_string());
        }
    }

    stack
}

/// 从 Cargo.toml 提取 Rust 框架名。
fn extract_rust_frameworks(root: &Path, stack: &mut ProjectStack) {
    let content = match std::fs::read_to_string(root.join("Cargo.toml")) {
        Ok(c) => c,
        Err(_) => return,
    };

    let known_frameworks = [
        "tokio",
        "async-std",
        "axum",
        "actix-web",
        "rocket",
        "warp",
        "serde",
        "clap",
        "tracing",
        "log",
        "anyhow",
        "thiserror",
        "reqwest",
        "hyper",
        "tonic",
        "sqlx",
        "diesel",
        "sea-orm",
    ];

    for fw in &known_frameworks {
        if content.contains(fw) {
            stack.frameworks.push(fw.to_string());
        }
    }
}

/// 检测 Node.js 包管理器。
fn detect_node_package_manager(root: &Path, stack: &mut ProjectStack) {
    if root.join("pnpm-lock.yaml").exists() {
        stack.package_managers.push("pnpm".to_string());
    } else if root.join("yarn.lock").exists() {
        stack.package_managers.push("yarn".to_string());
    } else {
        stack.package_managers.push("npm".to_string());
    }
}

/// 从 package.json 提取 Node.js 框架名。
fn extract_node_frameworks(root: &Path, stack: &mut ProjectStack) {
    let content = match std::fs::read_to_string(root.join("package.json")) {
        Ok(c) => c,
        Err(_) => return,
    };

    let known_frameworks = [
        "react", "next", "vue", "nuxt", "angular", "svelte", "solid", "express", "fastify", "koa",
        "hono", "nest", "vite", "webpack", "esbuild", "rollup",
    ];

    for fw in &known_frameworks {
        if content.contains(fw) {
            stack.frameworks.push(fw.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_rust_project() {
        let stack = detect_stack(Path::new("."));
        assert!(
            stack.languages.contains(&"Rust".to_string()),
            "should detect Rust"
        );
        assert!(
            stack.package_managers.contains(&"cargo".to_string()),
            "should detect cargo"
        );
        // 本项目用了 clap
        assert!(
            stack.frameworks.contains(&"clap".to_string()),
            "should detect clap"
        );
    }

    #[test]
    fn test_stack_summary() {
        let stack = ProjectStack {
            languages: vec!["Rust".to_string()],
            package_managers: vec!["cargo".to_string()],
            frameworks: vec!["clap".to_string()],
        };
        let summary = stack.summary();
        assert!(summary.contains("Rust"));
        assert!(summary.contains("cargo"));
    }

    #[test]
    fn test_stack_empty() {
        let stack = ProjectStack::default();
        assert!(stack.is_empty());
        assert_eq!(stack.summary(), "Unknown");
    }
}

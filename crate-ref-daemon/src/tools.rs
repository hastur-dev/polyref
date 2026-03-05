use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Run cargo fmt --check, cargo check, and cargo clippy in parallel.
/// Returns results in deterministic order: [fmt, check, clippy].
pub async fn run_all(project_root: &str) -> Vec<ToolResult> {
    let (fmt, check, clippy) = tokio::join!(
        run_tool("cargo", &["fmt", "--check"], project_root),
        run_tool(
            "cargo",
            &["check", "--message-format=json"],
            project_root
        ),
        run_tool(
            "cargo",
            &["clippy", "--message-format=json", "--", "-D", "warnings"],
            project_root,
        ),
    );
    vec![fmt, check, clippy]
}

async fn run_tool(program: &str, args: &[&str], cwd: &str) -> ToolResult {
    let name = format!("{} {}", program, args.join(" "));
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .await;

    match output {
        Ok(o) => ToolResult {
            tool: name,
            success: o.status.success(),
            stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
            exit_code: o.status.code().unwrap_or(-1),
        },
        Err(e) => ToolResult {
            tool: name,
            success: false,
            stdout: String::new(),
            stderr: e.to_string(),
            exit_code: -1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_tool_true() {
        #[cfg(unix)]
        let result = run_tool("true", &[], ".").await;
        #[cfg(windows)]
        let result = run_tool("cmd", &["/c", "exit", "0"], ".").await;
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_run_tool_false() {
        #[cfg(unix)]
        let result = run_tool("false", &[], ".").await;
        #[cfg(windows)]
        let result = run_tool("cmd", &["/c", "exit", "1"], ".").await;
        assert!(!result.success);
        assert_ne!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_tool_result_name_recorded() {
        #[cfg(unix)]
        let result = run_tool("echo", &["hello"], ".").await;
        #[cfg(windows)]
        let result = run_tool("cmd", &["/c", "echo", "hello"], ".").await;
        assert!(!result.tool.is_empty());
    }

    #[tokio::test]
    async fn test_run_all_returns_three_results() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cargo_toml = r#"[package]
name = "test_proj"
version = "0.1.0"
edition = "2021"
"#;
        std::fs::write(tmp.path().join("Cargo.toml"), cargo_toml).unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}\n").unwrap();

        let results = run_all(tmp.path().to_str().unwrap()).await;
        assert_eq!(results.len(), 3);
        assert!(results[0].tool.contains("fmt"));
        assert!(results[1].tool.contains("check"));
        assert!(results[2].tool.contains("clippy"));
    }
}

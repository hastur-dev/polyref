use std::path::Path;
use std::process::Command;

/// Result of delegating to the TypeScript checker.
#[derive(Debug)]
pub struct TsBridgeResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Run the TypeScript checker via subprocess.
///
/// Expects `node dist/cli.js` to be available from the `ts_checker_dir`.
/// Falls back gracefully if node is not available.
pub fn run_ts_checker(
    source_path: &str,
    refs_dir: &str,
    ts_checker_dir: &Path,
    enforce: bool,
    output_format: &str,
) -> anyhow::Result<TsBridgeResult> {
    let cli_script = ts_checker_dir.join("dist/cli.js");

    if !cli_script.exists() {
        anyhow::bail!(
            "TypeScript checker not built: {} not found. Run `npm run build` in {}",
            cli_script.display(),
            ts_checker_dir.display()
        );
    }

    let mut cmd = Command::new("node");
    cmd.arg(&cli_script)
        .arg(source_path)
        .arg("--refs")
        .arg(refs_dir)
        .arg("--output-format")
        .arg(output_format);

    if enforce {
        cmd.arg("--enforce");
    }

    let output = cmd.output().map_err(|e| {
        anyhow::anyhow!("failed to run node: {} — is Node.js installed?", e)
    })?;

    Ok(TsBridgeResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// Run the TypeScript checker from stdin.
pub fn run_ts_checker_stdin(
    source_content: &str,
    refs_dir: &str,
    ts_checker_dir: &Path,
    enforce: bool,
    output_format: &str,
) -> anyhow::Result<TsBridgeResult> {
    let cli_script = ts_checker_dir.join("dist/cli.js");

    if !cli_script.exists() {
        anyhow::bail!(
            "TypeScript checker not built: {} not found",
            cli_script.display()
        );
    }

    let mut cmd = Command::new("node");
    cmd.arg(&cli_script)
        .arg("--from-stdin")
        .arg("--refs")
        .arg(refs_dir)
        .arg("--output-format")
        .arg(output_format);

    if enforce {
        cmd.arg("--enforce");
    }

    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        anyhow::anyhow!("failed to spawn node: {}", e)
    })?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(source_content.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    Ok(TsBridgeResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// Check if the TypeScript checker is available.
pub fn is_ts_checker_available(ts_checker_dir: &Path) -> bool {
    ts_checker_dir.join("dist/cli.js").exists()
}

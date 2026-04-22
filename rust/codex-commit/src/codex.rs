use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use error_stack::{Report, ResultExt};

use crate::error::{AppError, AppResult};

pub fn ensure_codex_available() -> AppResult<()> {
    let output = Command::new("codex")
        .arg("--version")
        .output()
        .change_context(AppError::RepoEnvironment)
        .attach("Failed to locate `codex` on PATH")?;

    if output.status.success() {
        return Ok(());
    }

    Err(Report::new(AppError::RepoEnvironment).attach(format!(
        "`codex --version` failed: {}",
        stderr_text(&output.stderr)
    )))
}

pub fn run_codex(
    prompt: &str,
    schema_path: &Path,
    output_path: &Path,
    log_path: &Path,
) -> AppResult<()> {
    let output = Command::new("codex")
        .arg("exec")
        .arg("--ephemeral")
        .arg("--sandbox")
        .arg("read-only")
        .arg("-c")
        .arg("model_reasoning_effort=\"low\"")
        .arg("--output-schema")
        .arg(schema_path)
        .arg("-o")
        .arg(output_path)
        .arg(prompt)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .change_context(AppError::Codex)
        .attach("Failed to start `codex exec`")?;

    let log_contents = build_log_contents(&output.stdout, &output.stderr);
    fs::write(log_path, log_contents)
        .change_context(AppError::Codex)
        .attach(format!(
            "Failed to write log file at {}",
            log_path.display()
        ))?;

    io::stdout()
        .write_all(&output.stdout)
        .change_context(AppError::Codex)
        .attach("Failed to forward codex stdout")?;
    io::stderr()
        .write_all(&output.stderr)
        .change_context(AppError::Codex)
        .attach("Failed to forward codex stderr")?;

    if output.status.success() {
        return Ok(());
    }

    Err(Report::new(AppError::Codex).attach(format!(
        "`codex exec` failed. See log at {}. stderr: {}",
        log_path.display(),
        stderr_text(&output.stderr)
    )))
}

fn build_log_contents(stdout: &[u8], stderr: &[u8]) -> Vec<u8> {
    let mut log = Vec::new();
    log.extend_from_slice(b"[stdout]\n");
    log.extend_from_slice(stdout);
    if !stdout.ends_with(b"\n") {
        log.push(b'\n');
    }
    log.extend_from_slice(b"[stderr]\n");
    log.extend_from_slice(stderr);
    if !stderr.ends_with(b"\n") {
        log.push(b'\n');
    }
    log
}

fn stderr_text(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr).trim().to_string();
    if text.is_empty() {
        "codex returned a non-zero exit status".to_string()
    } else {
        text
    }
}

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use error_stack::{Report, ResultExt};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedCommit {
    pub sha: String,
    pub display: String,
}

pub fn repo_root() -> AppResult<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .change_context(AppError::RepoEnvironment)
        .attach("Failed to execute git while resolving the repository root")?;

    if !output.status.success() {
        return Err(Report::new(AppError::RepoEnvironment).attach(format!(
            "Not inside a git repository: {}",
            stderr_text(&output)
        )));
    }

    let root = stdout_text(&output);
    if root.is_empty() {
        return Err(
            Report::new(AppError::RepoEnvironment).attach("git did not return a repository root")
        );
    }

    Ok(PathBuf::from(root))
}

pub fn current_staged_paths(repo_root: &Path) -> AppResult<Vec<String>> {
    let output = git_output(repo_root, ["diff", "--cached", "--name-only"])?;
    Ok(lines(&stdout_text(&output)))
}

pub fn add_paths(repo_root: &Path, paths: &[String]) -> AppResult<()> {
    let mut command = git_command(repo_root);
    command.arg("add").arg("--").args(paths);
    run_git_status(command, AppError::Git, "Failed to stage proposed files")
}

pub fn commit_with_message_file(
    repo_root: &Path,
    message_file: &Path,
    color_output: bool,
) -> AppResult<CreatedCommit> {
    let mut command = git_command(repo_root);
    command.arg("commit").arg("-F").arg(message_file);
    run_git_status(command, AppError::Git, "Failed to create git commit")?;

    let sha = stdout_text(&git_output(repo_root, ["rev-parse", "HEAD"])?);
    let color_arg = commit_summary_color_arg(color_output);
    let display = stdout_text(&git_output(
        repo_root,
        [
            "log",
            "-1",
            "--stat",
            "--format=fuller",
            color_arg,
            sha.as_str(),
        ],
    )?);

    Ok(CreatedCommit { sha, display })
}

pub fn resolve_editor(repo_root: &Path) -> String {
    let mut command = git_command(repo_root);
    command.args(["var", "GIT_EDITOR"]);

    if let Ok(output) = command.output() {
        if output.status.success() {
            let editor = stdout_text(&output);
            if !editor.is_empty() {
                return editor;
            }
        }
    }

    env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "vi".to_string())
}

pub fn sorted_paths(paths: &[String]) -> Vec<String> {
    let mut sorted = paths.to_vec();
    sorted.sort_unstable();
    sorted
}

pub fn staged_sets_match(current_staged: &[String], proposed: &[String]) -> bool {
    sorted_paths(current_staged) == sorted_paths(proposed)
}

fn commit_summary_color_arg(color_output: bool) -> &'static str {
    if color_output {
        "--color=always"
    } else {
        "--color=never"
    }
}

fn git_output<const N: usize>(repo_root: &Path, args: [&str; N]) -> AppResult<Output> {
    git_command(repo_root)
        .args(args)
        .output()
        .change_context(AppError::Git)
        .attach(format!("Failed to execute git {:?}", args))
        .and_then(|output| {
            if output.status.success() {
                Ok(output)
            } else {
                Err(Report::new(AppError::Git).attach(format!(
                    "git {:?} failed: {}",
                    args,
                    stderr_text(&output)
                )))
            }
        })
}

fn run_git_status(mut command: Command, context: AppError, message: &str) -> AppResult<()> {
    let output = command
        .output()
        .change_context(context)
        .attach(format!("Failed to execute {}", message))?;

    if output.status.success() {
        return Ok(());
    }

    Err(Report::new(context).attach(format!("{}: {}", message, stderr_text(&output))))
}

fn git_command(repo_root: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(repo_root);
    command
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn stderr_text(output: &Output) -> String {
    let text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if text.is_empty() {
        "git command returned a non-zero exit status".to_string()
    } else {
        text
    }
}

fn lines(input: &str) -> Vec<String> {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod tests;

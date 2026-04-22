use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use error_stack::{Report, ResultExt};
use tempfile::tempdir;

use crate::assets::AssetPaths;
use crate::cli::Cli;
use crate::codex;
use crate::error::{AppError, AppResult};
use crate::git;
use crate::message;
use crate::prompt;
use crate::proposal::{Proposal, ProposalAlternative, ProposalStatus};

pub fn run(cli: Cli) -> AppResult<()> {
    let repo_root = git::repo_root()?;
    let assets = AssetPaths::resolve()?;
    let skill_text = assets.read_skill_text()?;
    codex::ensure_codex_available()?;

    let prompt = prompt::build_prompt(&skill_text, &cli.extra_context());
    let tempdir = tempdir()
        .change_context(AppError::Interaction)
        .attach("Failed to create temporary working directory")?;

    install_ctrlc_cleanup(tempdir.path().to_path_buf())?;

    let output_file = tempdir.path().join("proposal.json");
    let message_file = tempdir.path().join("COMMIT_EDITMSG");
    let log_file = tempdir.path().join("codex.log");

    codex::run_codex(&prompt, &assets.schema_path, &output_file, &log_file)?;

    let proposal = Proposal::from_path(&output_file)?;
    proposal.validate()?;

    match proposal.status {
        ProposalStatus::Ready => {}
        ProposalStatus::SplitRequired => {
            println!("{}", proposal.summary);
            print_alternatives(&proposal.alternatives);
            return Ok(());
        }
        ProposalStatus::NothingToCommit => {
            println!("{}", proposal.summary);
            return Ok(());
        }
    }

    let commit = proposal
        .commit
        .as_ref()
        .ok_or_else(|| Report::new(AppError::Proposal).attach("Ready proposal missing commit"))?;

    message::write_commit_message(&message_file, commit)?;

    let current_staged = git::current_staged_paths(&repo_root)?;
    if !current_staged.is_empty() && !git::staged_sets_match(&current_staged, &proposal.stage_paths)
    {
        return Err(Report::new(AppError::Git).attach(format!(
            "Proposal does not match the current staged set; refusing to add files.\nCurrently staged files:\n{}\n\nProposed files:\n{}",
            format_path_list(&current_staged),
            format_path_list(&proposal.stage_paths)
        )));
    }

    loop {
        print_proposal(&proposal, &message_file)?;
        match prompt_for_action()? {
            PromptAction::Commit => {
                if current_staged.is_empty() {
                    git::add_paths(&repo_root, &proposal.stage_paths)?;
                }
                git::commit_with_message_file(&repo_root, &message_file)?;
                return Ok(());
            }
            PromptAction::Edit => {
                open_editor(&repo_root, &message_file)?;
            }
            PromptAction::Retry => {
                println!("Press Enter or y to commit, n to edit, or Ctrl+C to cancel.");
            }
        }
    }
}

fn print_proposal(proposal: &Proposal, message_file: &Path) -> AppResult<()> {
    if !proposal.summary.trim().is_empty() {
        println!("{}\n", proposal.summary);
    }

    println!("Proposed files:");
    if proposal.stage_paths.is_empty() {
        println!("  (none)");
    } else {
        for path in &proposal.stage_paths {
            println!("  {path}");
        }
    }

    println!("\nProposed commit message:");
    println!("---");
    let message = fs::read_to_string(message_file)
        .change_context(AppError::Interaction)
        .attach(format!(
            "Failed to read commit message file at {}",
            message_file.display()
        ))?;
    print!("{message}");
    if !message.ends_with('\n') {
        println!();
    }
    println!("---");
    Ok(())
}

fn print_alternatives(alternatives: &[ProposalAlternative]) {
    if alternatives.is_empty() {
        return;
    }

    println!("\nSuggested split commits:");
    for (index, alternative) in alternatives.iter().enumerate() {
        println!(
            "\n{}. {}",
            index + 1,
            if alternative.summary.trim().is_empty() {
                format!("Alternative {}", index + 1)
            } else {
                alternative.summary.clone()
            }
        );
        if let Some(subject) = &alternative.commit_subject {
            if !subject.trim().is_empty() {
                println!("   Commit: {subject}");
            }
        }
        if !alternative.stage_paths.is_empty() {
            println!("   Files:");
            for path in &alternative.stage_paths {
                println!("     {path}");
            }
        }
    }
}

fn prompt_for_action() -> AppResult<PromptAction> {
    print!("Commit with this message? [Y/n] ");
    io::stdout()
        .flush()
        .change_context(AppError::Interaction)
        .attach("Failed to flush prompt to stdout")?;

    let mut action = String::new();
    io::stdin()
        .read_line(&mut action)
        .change_context(AppError::Interaction)
        .attach("Failed to read interactive input")?;

    let normalized = action.trim().to_ascii_lowercase();
    let action = match normalized.as_str() {
        "" | "y" | "yes" => PromptAction::Commit,
        "n" | "no" => PromptAction::Edit,
        _ => PromptAction::Retry,
    };

    Ok(action)
}

fn open_editor(repo_root: &Path, message_file: &Path) -> AppResult<()> {
    let editor = git::resolve_editor(repo_root);
    let command = format!("{} \"$1\"", editor);

    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .arg("codex-commit-editor")
        .arg(message_file)
        .output()
        .change_context(AppError::Interaction)
        .attach(format!("Failed to launch editor `{editor}`"))?;

    if output.status.success() {
        return Ok(());
    }

    Err(Report::new(AppError::Interaction).attach(format!(
        "Editor `{editor}` failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

fn install_ctrlc_cleanup(tempdir_path: PathBuf) -> AppResult<()> {
    let cleaned = Arc::new(Mutex::new(false));
    let handler_path = tempdir_path.clone();
    let handler_cleaned = Arc::clone(&cleaned);

    ctrlc::set_handler(move || {
        if let Ok(mut already_cleaned) = handler_cleaned.lock() {
            if !*already_cleaned {
                let _ = fs::remove_dir_all(&handler_path);
                *already_cleaned = true;
            }
        }
        std::process::exit(130);
    })
    .change_context(AppError::Interaction)
    .attach(format!(
        "Failed to install Ctrl+C handler for {}",
        tempdir_path.display()
    ))
}

fn format_path_list(paths: &[String]) -> String {
    paths
        .iter()
        .map(|path| format!("  {path}"))
        .collect::<Vec<_>>()
        .join("\n")
}

enum PromptAction {
    Commit,
    Edit,
    Retry,
}

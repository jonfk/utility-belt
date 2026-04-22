use std::fs;
use std::path::Path;

use error_stack::ResultExt;

use crate::error::{AppError, AppResult};
use crate::proposal::ProposalCommit;

pub fn build_commit_message(commit: &ProposalCommit) -> String {
    let subject = commit.subject.trim();
    let body_paragraphs: Vec<&str> = commit
        .body_paragraphs
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .collect();

    let mut message = String::new();
    message.push_str(subject);
    message.push('\n');

    if !body_paragraphs.is_empty() {
        message.push('\n');
        for paragraph in body_paragraphs {
            message.push_str(paragraph);
            message.push_str("\n\n");
        }
    }

    message
}

pub fn write_commit_message(path: &Path, commit: &ProposalCommit) -> AppResult<()> {
    fs::write(path, build_commit_message(commit))
        .change_context(AppError::Interaction)
        .attach(format!(
            "Failed to write commit message to {}",
            path.display()
        ))
}

#[cfg(test)]
#[path = "message_tests.rs"]
mod tests;

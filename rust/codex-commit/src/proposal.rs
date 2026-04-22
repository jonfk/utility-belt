use std::fs;
use std::path::Path;

use error_stack::{Report, ResultExt};
use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Ready,
    SplitRequired,
    NothingToCommit,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProposalCommit {
    pub subject: String,
    pub body_paragraphs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProposalAlternative {
    pub summary: String,
    pub commit_subject: Option<String>,
    pub stage_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Proposal {
    pub status: ProposalStatus,
    pub summary: String,
    pub stage_paths: Vec<String>,
    pub commit: Option<ProposalCommit>,
    pub alternatives: Vec<ProposalAlternative>,
}

impl Proposal {
    pub fn from_path(path: &Path) -> AppResult<Self> {
        let raw = fs::read_to_string(path)
            .change_context(AppError::Proposal)
            .attach(format!(
                "Failed to read proposal file at {}",
                path.display()
            ))?;

        serde_json::from_str::<Self>(&raw)
            .change_context(AppError::Proposal)
            .attach(format!(
                "Failed to parse proposal JSON from {}",
                path.display()
            ))
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.summary.trim().is_empty() {
            return Err(
                Report::new(AppError::Proposal).attach("Proposal summary must not be empty")
            );
        }

        match self.status {
            ProposalStatus::Ready => self.validate_ready(),
            ProposalStatus::SplitRequired => self.validate_split_required(),
            ProposalStatus::NothingToCommit => self.validate_nothing_to_commit(),
        }
    }

    fn validate_ready(&self) -> AppResult<()> {
        if self.stage_paths.is_empty() {
            return Err(Report::new(AppError::Proposal)
                .attach("Ready proposal must include at least one stage path"));
        }

        let commit = self.commit.as_ref().ok_or_else(|| {
            Report::new(AppError::Proposal).attach("Ready proposal must include a commit object")
        })?;

        if commit.subject.trim().is_empty() {
            return Err(Report::new(AppError::Proposal).attach("Commit subject must not be empty"));
        }

        Ok(())
    }

    fn validate_split_required(&self) -> AppResult<()> {
        if self.commit.is_some() {
            return Err(Report::new(AppError::Proposal)
                .attach("split_required proposal must not include a commit"));
        }

        if !self.stage_paths.is_empty() {
            return Err(Report::new(AppError::Proposal)
                .attach("split_required proposal must not include stage_paths"));
        }

        if self.alternatives.is_empty() {
            return Err(Report::new(AppError::Proposal)
                .attach("split_required proposal must include alternatives"));
        }

        Ok(())
    }

    fn validate_nothing_to_commit(&self) -> AppResult<()> {
        if self.commit.is_some() {
            return Err(Report::new(AppError::Proposal)
                .attach("nothing_to_commit proposal must not include a commit"));
        }

        if !self.stage_paths.is_empty() {
            return Err(Report::new(AppError::Proposal)
                .attach("nothing_to_commit proposal must not include stage_paths"));
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "proposal_tests.rs"]
mod tests;

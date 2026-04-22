use error_stack::Report;
use thiserror::Error;

pub type AppResult<T> = Result<T, Report<AppError>>;

#[derive(Debug, Clone, Copy, Error)]
pub enum AppError {
    #[error("Repository or environment check failed")]
    RepoEnvironment,

    #[error("Runtime assets are unavailable")]
    Assets,

    #[error("Codex execution failed")]
    Codex,

    #[error("Codex proposal was invalid")]
    Proposal,

    #[error("Git command failed")]
    Git,

    #[error("User interaction failed")]
    Interaction,
}

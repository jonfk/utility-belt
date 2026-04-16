use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Ghostty query failed")]
    Ghostty,

    #[error("AppleScript execution failed")]
    AppleScript,

    #[error("Failed to parse Ghostty output")]
    Parse,

    #[error("Failed to render output")]
    Output,
}

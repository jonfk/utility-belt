use serde::Deserialize;
use std::process::Command;
use tracing::{event, span, Level};

use crate::error::CmdqError;

#[derive(Debug, Deserialize)]
pub struct Record {
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "title")]
    pub title: String,
}

pub fn execute(url: &str, title: &str) -> Result<(), CmdqError> {
    let span = span!(Level::INFO, "yt-dlp execute", url, title);
    let _enter = span.enter();

    let filename = format!("{} [%(id)s].%(ext)s", clean_title(title));
    validate_filename(&filename)?;

    event!(Level::INFO, "executing");
    let args = vec![url.to_string(), "-o".to_string(), filename.clone()];

    let output = Command::new("yt-dlp").args(&args).output().map_err(|err| {
        CmdqError::ProcessExecuteError {
            err: err,
            program: "yt-dlp".to_string(),
            args: args,
        }
    })?;

    if output.status.success() {
        event!(Level::INFO, "execution succeeded");
        Ok(())
    } else {
        let error = CmdqError::ProcessExecuteOutputError {
            stdout: String::from_utf8(output.stdout)
                .map_err(|_err| CmdqError::ProcessExecuteOutputNotUtf8Error)?,
            stderr: String::from_utf8(output.stderr)
                .map_err(|_err| CmdqError::ProcessExecuteOutputNotUtf8Error)?,
        };
        event!(Level::INFO, message = "execution failed", ?error);
        Err(error)
    }
}

fn clean_title(title: &str) -> String {
    title
        .replace("/", "_")
        .replace("\\", "_")
        .replace("+", "_")
        .replace(":", " ")
}

fn validate_filename(filename: &str) -> Result<(), CmdqError> {
    if filename.bytes().len() > 255 {
        Err(CmdqError::FilenameTooLongError {
            filename: filename.to_string(),
        })
    } else {
        Ok(())
    }
}

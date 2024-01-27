use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};
use tracing::{event, Level};

use crate::error::CmdqError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "title")]
    pub title: String,
    pub dir: Option<String>,
}

pub fn execute(filepath: &Path, record: &Record) -> Result<(), CmdqError> {
    let title = &record.title;
    let url = record.url.clone();

    let target_dir = target_dir(filepath, &record.dir)?;
    event!(Level::INFO, target_dir = format!("{:?}", target_dir));

    let args = if title.trim().len() == 0 {
        vec![url.to_string()]
    } else {
        let filename = format!("{} [%(id)s].%(ext)s", clean_title(title));
        validate_filename(&filename)?;

        vec![url.to_string(), "-o".to_string(), filename.clone()]
    };

    let output = Command::new("yt-dlp")
        .args(&args)
        .current_dir(target_dir)
        .output()
        .map_err(|err| CmdqError::ProcessExecuteError {
            err: err,
            program: "yt-dlp".to_string(),
            args: args,
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let error = CmdqError::ProcessExecuteOutputError {
            stdout: String::from_utf8(output.stdout)
                .map_err(|_err| CmdqError::ProcessExecuteOutputNotUtf8Error)?,
            stderr: String::from_utf8(output.stderr)
                .map_err(|_err| CmdqError::ProcessExecuteOutputNotUtf8Error)?,
        };
        Err(error)
    }
}

fn target_dir(filepath: &Path, dir: &Option<String>) -> Result<PathBuf, CmdqError> {
    let input_dir = if let Some(parent) = filepath.parent() {
        if parent.is_absolute() {
            parent.to_path_buf()
        } else {
            let mut absolute_file_dir = env::current_dir()
                .map_err(|e| CmdqError::GetTargetDirFromCurrentDirError { source: e })?;
            absolute_file_dir.push(parent);
            absolute_file_dir
        }
    } else {
        env::current_dir().map_err(|e| CmdqError::GetTargetDirFromCurrentDirError { source: e })?
    };

    Ok(dir
        .as_ref()
        .map(|dir| {
            let dir_path = PathBuf::from(dir);
            if dir_path.is_absolute() {
                dir_path
            } else {
                let mut target_dir = input_dir.clone();
                target_dir.push(dir_path);
                target_dir
            }
        })
        .unwrap_or_else(|| input_dir))
}

fn clean_title(title: &str) -> String {
    title
        .replace("/", "_")
        .replace("\\", "_")
        .replace("+", "_")
        .replace(":", " ")
        .replace("?", " ")
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

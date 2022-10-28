use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CmdqError {
    #[error("Error executing `{program}` with args {}", .args.join(" "))]
    ProcessExecuteError {
        err: io::Error,
        program: String,
        args: Vec<String>,
    },

    #[error("The filename you are trying to create is too long. filename = `{filename}`")]
    FilenameTooLongError { filename: String },

    #[error("Process execution completed with error: {} {}", .stdout, .stderr)]
    ProcessExecuteOutputError { stdout: String, stderr: String },

    #[error("Stdout or stderr cannot be processed as text")]
    ProcessExecuteOutputNotUtf8Error,

    #[error("Could not open File `{}`", filepath)]
    FileOpenError { source: io::Error, filepath: String },

    #[error("Could not deserialize record on line `{}`, with error = `{}`", .line_number, source)]
    CsvDeserializeError {
        source: csv::Error,
        line_number: usize,
    },

    #[error("Could not create error file `{}`: {}", filepath.display(), source)]
    CreateErrorFileError {
        source: io::Error,
        filepath: PathBuf,
    },

    #[error("Could write error to error file `{}`: {}", filepath.display(), source)]
    WriteToErrorFileError {
        source: csv::Error,
        filepath: PathBuf,
    },

    #[error("Could write error file `{}`: {}", filepath.display(), source)]
    WriteErrorFileError {
        source: io::Error,
        filepath: PathBuf,
    },
}

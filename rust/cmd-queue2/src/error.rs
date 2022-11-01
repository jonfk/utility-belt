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

    #[error("Could not open File `{}`", filepath.display())]
    FileOpenError {
        source: io::Error,
        filepath: PathBuf,
    },

    #[error("Could not deserialize record with error = `{}`", source)]
    CsvDeserializeError { source: csv::Error },

    #[error("Could not create error file `{}`: {}", filepath.display(), source)]
    CreateErrorFileError {
        source: io::Error,
        filepath: PathBuf,
    },

    #[error("Could not write error to error file `{}`: {}", filepath.display(), source)]
    WriteToErrorFileError {
        source: csv::Error,
        filepath: PathBuf,
    },

    #[error("Could not write error file `{}`: {}", filepath.display(), source)]
    WriteErrorFileError {
        source: io::Error,
        filepath: PathBuf,
    },

    #[error("Error removing input file `{}`: {}", filepath.display(), source)]
    RemoveInputFileError {
        source: io::Error,
        filepath: PathBuf,
    },
}

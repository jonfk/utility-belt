use std::num::ParseIntError;

use nix::errno::Errno;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {}

#[derive(Error, Debug)]
pub enum CmdqClientError {
    #[error("Error reading PID file of server at {}. {}", .0, .1)]
    ReadServerPidFile(String, std::io::Error),

    #[error("Error parsing PID of server with {}", .0)]
    ParseServerPid(ParseIntError),

    #[error("Error sending kill signal to server pid {} with {}", .0, .1)]
    KillServer(i32, Errno),

    #[error("Error sending HTTP request with {}", .0)]
    HttpClientError(reqwest::Error),

    #[error("Error deserializing HTTP response with {}", .0)]
    ResponseDeserializationError(reqwest::Error),

    #[error("Error parsing server host {}. {}", .0, .1)]
    ServerHostUrlParseError(String, url::ParseError),
}

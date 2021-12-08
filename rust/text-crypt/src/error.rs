use thiserror::Error;

use crate::crypto::{CryptoDecryptError, CryptoEncryptError};

#[derive(Error, Debug)]
pub enum CheckError {
    #[error("Unencrypted file: {}", .0)]
    UnencryptedFile(String),

    #[error("Error reading file: {} Error: {}", .0, .1)]
    ReadFile(String, std::io::Error),

    #[error("Error walking dir: {} Error: {}", .0, .1)]
    WalkDir(String, walkdir::Error),

    #[error("Error parsing file: {} Error: {}", .0, .1)]
    ParseCryptFile(String, ParseError),
}

// TODO implement Debug manually
#[derive(Debug)]
pub struct CheckErrors {
    errors: Vec<CheckError>,
}

impl CheckErrors {
    pub fn new(errors: Vec<CheckError>) -> Self {
        CheckErrors { errors }
    }
}

#[derive(Error, Debug)]
pub enum EncryptError {
    #[error("Error parsing file: {} Error: {}", .0, .1)]
    ParseCryptFile(String, ParseError),

    #[error("Error reading file: {} Error: {}", .0, .1)]
    ReadFile(String, std::io::Error),

    #[error("Error writing file: {} Error: {}", .0, .1)]
    WriteFile(String, std::io::Error),

    #[error("Error walking dir: {} Error: {}", .0, .1)]
    WalkDir(String, walkdir::Error),

    #[error(transparent)]
    Encryption(#[from] CryptoEncryptError),
}

// TODO implement Debug manually
#[derive(Debug)]
pub struct EncryptErrors {
    errors: Vec<EncryptError>,
}

impl EncryptErrors {
    pub fn new(errors: Vec<EncryptError>) -> Self {
        EncryptErrors { errors }
    }
}

#[derive(Error, Debug)]
pub enum DecryptError {
    #[error("Error parsing file: {} Error: {}", .0, .1)]
    ParseCryptFile(String, ParseError),

    #[error("Error reading file: {} Error: {}", .0, .1)]
    ReadFile(String, std::io::Error),

    #[error("Error writing file: {} Error: {}", .0, .1)]
    WriteFile(String, std::io::Error),

    #[error("Error walking dir: {} Error: {}", .0, .1)]
    WalkDir(String, walkdir::Error),
    #[error(transparent)]
    Decryption(#[from] CryptoDecryptError),
}

#[derive(Debug)]
pub struct DecryptErrors {
    errors: Vec<DecryptError>,
}

impl DecryptErrors {
    pub fn new(errors: Vec<DecryptError>) -> Self {
        DecryptErrors { errors }
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error(
        "CRYPT Header does not have right number of arguments at line {}",
        .0
    )]
    InvalidHeader(usize),

    #[error(
        "A CRYPT block cannot have more than 1 header at line {}",
        .0
    )]
    MultipleHeaders(usize),

    #[error(
        "A CRYPT END Header was encountered with no start at line {}",
        .0
    )]
    EndHeaderWithNoStart(usize),

    #[error(
        "A empty CRYPT block was encountered at line {}",
        .0
    )]
    EmptyCryptBlock(usize),

    #[error(
        "A CRYPT END was encountered with no start at line {}",
        .0
    )]
    EndWithNoStart(usize),

    #[error("A CRYPT START was encountered with no end")]
    StartWithNoEnd,
}

use std::fmt;

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
pub struct CheckErrors {
    errors: Vec<CheckError>,
}

impl CheckErrors {
    pub fn new(errors: Vec<CheckError>) -> Self {
        CheckErrors { errors }
    }
}

impl fmt::Debug for CheckErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\nErrors encountered checking files\n")?;
        self.errors
            .iter()
            .map(|error| write!(f, "{}\n", error))
            .collect::<fmt::Result>()?;
        Ok(())
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

pub struct EncryptErrors {
    errors: Vec<EncryptError>,
}

impl EncryptErrors {
    pub fn new(errors: Vec<EncryptError>) -> Self {
        EncryptErrors { errors }
    }
}

impl fmt::Debug for EncryptErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\nErrors encountered encrypting files\n")?;
        self.errors
            .iter()
            .map(|error| write!(f, "{}\n", error))
            .collect()
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

pub struct DecryptErrors {
    errors: Vec<DecryptError>,
}

impl DecryptErrors {
    pub fn new(errors: Vec<DecryptError>) -> Self {
        DecryptErrors { errors }
    }
}

impl fmt::Debug for DecryptErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\nErrors encountered decrypting files\n")?;
        self.errors
            .iter()
            .map(|error| write!(f, "{}\n", error))
            .collect()
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("The number of Begin and End Crypt blocks don't match")]
    MismatchNumStartEndCryptBlocks,

    #[error("Encountered and End before a Begin at line {}", .0)]
    EndBeforeBegin(usize),

    #[error("Blocks cannot be nested. Encountered a second Begin before End of block at line {}", .0)]
    BeginBeforeEnd(usize),

    #[error("Base64 decoding error: {}", .0)]
    Base64Decode(base64::DecodeError),

    #[error("MessagePack decoding error: {}", .0)]
    MessagePackDecode(rmp_serde::decode::Error),
}

#[derive(Error, Debug)]
pub enum EncryptedCryptEncodingError {
    #[error("MessagePack encoding error: {}", .0)]
    MessagePackEncode(rmp_serde::encode::Error),
}

pub mod cli;
pub mod crypto;
pub mod parse;

use std::fmt;

use std::{fs::File, io::Write};

use base64;
use chacha20poly1305::{
    aead::{Aead, NewAead},
    ChaCha20Poly1305, Key, Nonce,
};
use clap::{App, Arg, SubCommand};
use thiserror::Error;
use walkdir::WalkDir;

static START_DELIMITER: &'static str = "---BEGIN CRYPT---";
static END_HEADER_DELIMITER: &'static str = "---END CRYPT HEADER---";
static END_DELIMITER: &'static str = "---END CRYPT---";

const BASE64_CONFIG: base64::Config = base64::STANDARD_NO_PAD;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Block {
    Plaintext(String),
    Crypt(CryptBlock),
}

pub struct CryptFile {
    blocks: Vec<Block>,
}

impl fmt::Display for CryptFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.blocks
            .iter()
            .map(|block| match block {
                Block::Plaintext(text) => write!(f, "{}", text),
                Block::Crypt(crypt_block) => {
                    if crypt_block.has_algorithm_and_nonce() {
                        write!(
                            f,
                            "{}\n{};{}\n{}\n{}\n{}\n",
                            START_DELIMITER,
                            crypt_block.algorithm.as_ref().expect("missing algorithm"),
                            crypt_block.nonce.as_ref().expect("missing nonce"),
                            END_HEADER_DELIMITER,
                            crypt_block.ciphertext,
                            END_DELIMITER
                        )
                    } else {
                        write!(
                            f,
                            "{}\n{}\n{}\n",
                            START_DELIMITER, crypt_block.ciphertext, END_DELIMITER
                        )
                    }
                }
            })
            .collect()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CryptBlock {
    algorithm: Option<String>,
    nonce: Option<String>,
    ciphertext: String,
}

impl CryptBlock {
    fn is_encrypted(&self) -> bool {
        self.has_algorithm_and_nonce()
            && base64::decode_config(&self.ciphertext, BASE64_CONFIG).is_ok()
    }
    fn has_algorithm_and_nonce(&self) -> bool {
        self.algorithm.is_some() && self.nonce.is_some()
    }

    fn new_unencrypted(txt: &str) -> Self {
        CryptBlock {
            ciphertext: txt.to_string(),
            ..Default::default()
        }
    }
}

#[derive(Error, Debug)]
enum CryptError {
    #[error("unknown error")]
    Unknown,
}

#[derive(Error, Debug)]
enum CheckError {
    #[error("Unencrypted file: {}", .0)]
    UnencryptedFile(String),

    #[error("Error reading file: {} Error: {}", .0, .1)]
    ReadFile(String, std::io::Error),

    #[error("Error walking dir: {} Error: {}", .0, .1)]
    WalkDir(String, walkdir::Error),

    #[error("Error parsing file: {} Error: {}", .0, .1)]
    ParseCryptFile(String, ParseError),
}

#[derive(Debug)]
struct CheckErrors {
    errors: Vec<CheckError>,
}

#[derive(Error, Debug)]
enum EncryptError {
    #[error("Error reading file: {} Error: {}", .0, .1)]
    ReadFile(String, std::io::Error),

    #[error("Error walking dir: {} Error: {}", .0, .1)]
    WalkDir(String, walkdir::Error),
}

struct EncryptErrors {
    errors: Vec<EncryptError>,
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

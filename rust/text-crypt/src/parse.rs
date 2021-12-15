use std::fmt;

use crate::error::EncryptedCryptEncodingError;
use crate::error::ParseError;

use lazy_static::lazy_static;
use regex::Regex;
use rmp_serde;
use serde::{Deserialize, Serialize};

const BASE64_CONFIG: base64::Config = base64::STANDARD_NO_PAD;

const BEGIN_CRYPT_STR: &'static str = "BEGIN CRYPT";
const BEGIN_ENCRYPTED_CRYPT_ENCRYPTION_MARKER_STR: &'static str = "..";
const END_CRYPT_STR: &'static str = "END CRYPT";
const BEGIN_CRYPT_LEN: usize = BEGIN_CRYPT_STR.len();
const BEGIN_ENCRYPTED_CRYPT_LEN: usize =
    BEGIN_CRYPT_STR.len() + BEGIN_ENCRYPTED_CRYPT_ENCRYPTION_MARKER_STR.len();
const END_CRYPT_LEN: usize = END_CRYPT_STR.len();

lazy_static! {
    static ref IS_CRYPT_FILE_RE: Regex = Regex::new(r"(?i)BEGIN[\s\W\p{Punct}]*CRYPT").unwrap();
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct EncryptedCryptBlock {
    pub algorithm: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl EncryptedCryptBlock {
    pub fn from_str(input: &str) -> Result<Self, ParseError> {
        let trimmed_input = if input[..BEGIN_CRYPT_LEN]
            .to_uppercase()
            .starts_with(BEGIN_CRYPT_STR)
        {
            &input[BEGIN_ENCRYPTED_CRYPT_LEN..(input.len() - END_CRYPT_LEN)]
        } else {
            input
        };
        let input_bytes = base64::decode_config(trimmed_input, BASE64_CONFIG)
            .map_err(|e| ParseError::Base64Decode(e))?;
        let block: EncryptedCryptBlock =
            rmp_serde::from_read(&*input_bytes).map_err(|e| ParseError::MessagePackDecode(e))?;

        Ok(block)
    }

    pub fn to_ascii_armor(&self) -> Result<String, EncryptedCryptEncodingError> {
        let mut buf = Vec::new();
        // TODO: extract to `fn to_msg_pack()`?
        self.serialize(&mut rmp_serde::Serializer::new(&mut buf))
            .map_err(|e| EncryptedCryptEncodingError::MessagePackEncode(e))?;

        let encoded = base64::encode_config(buf, BASE64_CONFIG);
        Ok(format!(
            "{}{}{}{}",
            BEGIN_CRYPT_STR, BEGIN_ENCRYPTED_CRYPT_ENCRYPTION_MARKER_STR, encoded, END_CRYPT_STR
        ))
    }
}

#[test]
fn test_parsing_encrypted_crypt_block() {
    let block = EncryptedCryptBlock {
        algorithm: "test_algo".to_string(),
        nonce: b"nonce".to_vec(),
        ciphertext: b"this is some good ciphertext".to_vec(),
    };
    let armored = block.to_ascii_armor().unwrap();

    let parsed = EncryptedCryptBlock::from_str(&armored).unwrap();

    assert_eq!(parsed, block);
}

#[derive(Debug, PartialEq)]
pub enum Block {
    Plaintext(String),
    UnencryptedCryptBlock(String),
    EncryptedCryptBlock(EncryptedCryptBlock),
}

impl Block {
    pub fn is_encrypted(&self) -> bool {
        match self {
            Block::Plaintext(_) | Block::UnencryptedCryptBlock(_) => false,
            Block::EncryptedCryptBlock(_) => true,
        }
    }
}

#[derive(Debug)]
pub struct CryptFile {
    pub blocks: Vec<Block>,
}

impl fmt::Display for CryptFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.blocks
            .iter()
            .map(|block| match block {
                Block::Plaintext(text) => write!(f, "{}", text),
                Block::UnencryptedCryptBlock(text) => {
                    write!(f, "{}\n{}\n{}", BEGIN_CRYPT_STR, text, END_CRYPT_STR)
                }
                Block::EncryptedCryptBlock(block) => {
                    let ascii_armored = block.to_ascii_armor().map_err(|e| fmt::Error)?;
                    write!(f, "{}", ascii_armored)
                }
            })
            .collect()
    }
}

impl CryptFile {
    pub fn is_crypt_file(contents: &str) -> bool {
        IS_CRYPT_FILE_RE.is_match(contents)
    }

    pub fn from_str(contents: &str) -> Result<CryptFile, ParseError> {
        let mut blocks = Vec::new();

        // Replace with Regexes with case insensitive matching
        let crypt_block_starts: Vec<_> = contents.match_indices(BEGIN_CRYPT_STR).collect();
        let crypt_block_ends: Vec<_> = contents.match_indices(END_CRYPT_STR).collect();
        dbg!(&crypt_block_starts);
        dbg!(&crypt_block_ends);
        if crypt_block_starts.len() != crypt_block_ends.len() {
            return Err(ParseError::MismatchNumStartEndCryptBlocks);
        }

        if !crypt_block_starts.is_empty() && crypt_block_starts[0].0 != 0 {
            let first_crypt_block = crypt_block_starts[0].0;
            let plaintext = contents[0..first_crypt_block].to_string();
            blocks.push(Block::Plaintext(plaintext));
        }

        let mut prev_end = 0;
        for ((start_idx, _), (end_idx, _)) in crypt_block_starts
            .into_iter()
            .zip(crypt_block_ends.into_iter())
        {
            if end_idx < start_idx {
                return Err(ParseError::EndBeforeBegin(0));
            }
            if prev_end != 0 && prev_end < start_idx {
                return Err(ParseError::BeginBeforeEnd(0));
            }
            if prev_end != 0 {
                let plaintext = contents[(prev_end + END_CRYPT_STR.len())..start_idx].to_string();
                blocks.push(Block::Plaintext(plaintext));
            }

            let matching_contents = &contents[(start_idx + BEGIN_CRYPT_STR.len())..end_idx];
            let block =
                if matching_contents.starts_with(BEGIN_ENCRYPTED_CRYPT_ENCRYPTION_MARKER_STR) {
                    Block::EncryptedCryptBlock(EncryptedCryptBlock::from_str(
                        &matching_contents[BEGIN_ENCRYPTED_CRYPT_ENCRYPTION_MARKER_STR.len()..],
                    )?)
                } else {
                    Block::UnencryptedCryptBlock(matching_contents.trim().to_string())
                };
            blocks.push(block);
            prev_end = end_idx;
        }

        Ok(CryptFile { blocks })
    }
}

#[test]
fn test_from_str_unencrypted() {
    let contents = r#"hello worldBEGIN CRYPThelloworldencryptedEND CRYPTBEGIN CRYPTthisis a test\n\nEND CRYPT
"#;
    let parsed = CryptFile::from_str(contents).unwrap();
    dbg!(parsed);
}

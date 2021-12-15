pub mod cli;
pub mod crypto;
pub mod error;
pub mod parse;

use std::fmt;

use base64;

static START_DELIMITER: &'static str = "---BEGIN CRYPT---";
static END_HEADER_DELIMITER: &'static str = "---END CRYPT HEADER---";
static END_DELIMITER: &'static str = "---END CRYPT---";

static START_DELIMITER_WHOLE_REGEX: &'static str = r"(?i)^-*BEGIN[\s\W\p{Punct}]*CRYPT-*$";
static START_DELIMITER_CONTAINS_REGEX: &'static str = r"(?i)-*BEGIN[\s\W\p{Punct}]*CRYPT-*";
static END_HEADER_WHOLE_REGEX: &'static str = r"(?i)^-*END.*CRYPT[\s\W\p{Punct}]*HEADER-*$";
static END_HEADER_CONTAINS_REGEX: &'static str = r"(?i)-*END.*CRYPT[\s\W\p{Punct}]*HEADER-*";
static END_DELIMITER_WHOLE_REGEX: &'static str = r"(?i)^-*END[\s\W\p{Punct}]*CRYPT-*$";
static END_DELIMITER_CONTAINS_REGEX: &'static str = r"(?i)^-*END[\s\W\p{Punct}]*CRYPT-*$";

const BASE64_CONFIG: base64::Config = base64::STANDARD_NO_PAD;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Block {
    Plaintext(String),
    Crypt(CryptBlock),
}

#[derive(Debug)]
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
                        write!(f, "BEGIN CRYPT\n{}\nEND CRYPT\n", crypt_block.ciphertext)
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

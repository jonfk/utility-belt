use crate::error::ParseError;
use crate::Block;
use crate::CryptBlock;
use crate::CryptFile;
use crate::END_DELIMITER_CONTAINS_REGEX;
use crate::END_DELIMITER_WHOLE_REGEX;
use crate::END_HEADER_CONTAINS_REGEX;
use crate::END_HEADER_WHOLE_REGEX;
use crate::START_DELIMITER_CONTAINS_REGEX;
use crate::START_DELIMITER_WHOLE_REGEX;

use lazy_static::lazy_static;
use regex::Regex;
use regex::RegexSet;

lazy_static! {
    static ref START_RE: Regex = Regex::new(START_DELIMITER_WHOLE_REGEX).unwrap();
    static ref HEADER_RE: Regex = Regex::new(END_HEADER_WHOLE_REGEX).unwrap();
    static ref END_RE: Regex = Regex::new(END_DELIMITER_WHOLE_REGEX).unwrap();
    static ref ALL_DELIMITERS_CONTAINS_RE: RegexSet = RegexSet::new(&[
        START_DELIMITER_CONTAINS_REGEX,
        END_HEADER_CONTAINS_REGEX,
        END_DELIMITER_CONTAINS_REGEX
    ])
    .unwrap();
}

impl CryptFile {
    pub fn is_crypt_file(contents: &str) -> bool {
        ALL_DELIMITERS_CONTAINS_RE.is_match(contents)
    }

    pub fn from_str(contents: &str) -> Result<CryptFile, ParseError> {
        let mut current = String::new();
        let mut current_crypt_block = None;
        let mut blocks = Vec::new();

        for (line_idx, line) in contents.lines().enumerate() {
            let line_num = line_idx + 1;
            if START_RE.is_match(line.trim()) {
                if !current.is_empty() {
                    blocks.push(Block::Plaintext(current));
                }
                current = String::new();
                current_crypt_block = Some(CryptBlock::default());
            } else if HEADER_RE.is_match(line.trim()) {
                let header: Vec<_> = current.split(";").collect();
                if header.len() != 2 {
                    return Err(ParseError::InvalidHeader(line_num));
                }
                if let Some(current_crypt_block) = current_crypt_block.as_mut() {
                    if current_crypt_block.algorithm.is_some()
                        || current_crypt_block.nonce.is_some()
                    {
                        return Err(ParseError::MultipleHeaders(line_num));
                    }
                    current_crypt_block.algorithm = Some(header[0].to_string());
                    current_crypt_block.nonce = Some(header[1].trim().to_string());
                } else {
                    return Err(ParseError::EndHeaderWithNoStart(line_num));
                }
                current = String::new();
            } else if END_RE.is_match(line.trim()) {
                if current.trim().is_empty() {
                    return Err(ParseError::EmptyCryptBlock(line_num));
                }
                if let Some(mut current_crypt_block) = current_crypt_block {
                    current_crypt_block.ciphertext = current.trim_end().to_string();
                    blocks.push(Block::Crypt(current_crypt_block));
                } else {
                    return Err(ParseError::EndWithNoStart(line_num));
                }
                current = String::new();
                current_crypt_block = None;
            } else if ALL_DELIMITERS_CONTAINS_RE.is_match(line) {
                return Err(ParseError::DelimiterWithAdditionalText(line_num));
            } else {
                current.push_str(line);
                current.push('\n');
            }
        }

        if current_crypt_block.is_some() {
            return Err(ParseError::StartWithNoEnd);
        }
        if !current.trim().is_empty() {
            blocks.push(Block::Plaintext(current));
        }

        Ok(CryptFile { blocks })
    }
}

#[test]
fn test_parse_file() {
    let contents = r#"hello this is a test
test
testing
    ---BEGIN CRYPT---
hello
    ---END CRYPT---
blahblahblah
"#;

    let crypt_file = CryptFile::from_str(&contents).unwrap();
    assert_eq!(crypt_file.blocks.len(), 3);
    assert_eq!(
        crypt_file.blocks,
        vec![
            Block::Plaintext("hello this is a test\ntest\ntesting\n".to_string()),
            Block::Crypt(CryptBlock {
                algorithm: None,
                nonce: None,
                ciphertext: "hello".to_string()
            }),
            Block::Plaintext("blahblahblah\n".to_string())
        ]
    );
}

#[test]
fn test_parse_file2() {
    let contents = r"---BEGIN_CRYPT--
testhello
---END CRYPT---
";
    let crypt_file = CryptFile::from_str(contents).unwrap();
    assert_eq!(crypt_file.blocks.len(), 1);
    assert_eq!(
        crypt_file.blocks,
        vec![Block::Crypt(CryptBlock {
            ciphertext: "testhello".to_string(),
            ..Default::default()
        })]
    );
}

#[test]
fn test_parse_file3() {
    let contents = r"BEGIN TEST HELLO CRYPT
testhello
END CRYPT
";
    let crypt_file = CryptFile::from_str(contents);
    assert!(crypt_file.is_err());
    //assert!(crypt_file.unwrap_err().m);
}

#[test]
fn test_parse_file4() {
    let contents = r"BEGIN TEST HELLO CRYPT
testhello
END TEST CRYPT
";
    assert!(!CryptFile::is_crypt_file(contents));
}

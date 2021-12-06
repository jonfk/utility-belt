use crate::Block;
use crate::CryptBlock;
use crate::ParseError;
use crate::END_DELIMITER;
use crate::END_HEADER_DELIMITER;
use crate::{CryptFile, START_DELIMITER};

impl CryptFile {
    pub fn is_crypt_file(contents: &str) -> bool {
        contents.contains(START_DELIMITER.trim_matches('-'))
    }

    pub fn from_str(contents: &str) -> Result<CryptFile, ParseError> {
        let mut current = String::new();
        let mut current_crypt_block = None;
        let mut blocks = Vec::new();

        for (line_idx, line) in contents.lines().enumerate() {
            let line_num = line_idx + 1;
            if line
                .to_lowercase()
                .contains(&START_DELIMITER.trim_matches('-').to_lowercase())
            {
                blocks.push(Block::Plaintext(current));
                current = String::new();
                current_crypt_block = Some(CryptBlock::default());
            } else if line
                .to_lowercase()
                .contains(&END_HEADER_DELIMITER.trim_matches('-').to_lowercase())
            {
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
            } else if line
                .to_lowercase()
                .contains(&END_DELIMITER.trim_matches('-').to_lowercase())
            {
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
                ciphertext: "hello\n".to_string()
            }),
            Block::Plaintext("blahblahblah\n".to_string())
        ]
    );
}

use std::path::Path;

use thiserror::Error;

use crate::Block;
use crate::CryptBlock;
use crate::END_DELIMITER;
use crate::END_HEADER_DELIMITER;
use crate::{CryptFile, START_DELIMITER};

pub fn parse_file<T: AsRef<Path>>(filepath: T, contents: &str) -> CryptFile {
    let filepath = filepath.as_ref().to_string_lossy();
    let mut current = String::new();
    let mut current_crypt_block = None;
    let mut blocks = Vec::new();

    for line in contents.lines() {
        if line
            .to_lowercase()
            .contains(&START_DELIMITER.to_lowercase())
        {
            blocks.push(Block::Plaintext(current));
            current = String::new();
            current_crypt_block = Some(CryptBlock::default());
        } else if line
            .to_lowercase()
            .contains(&END_HEADER_DELIMITER.to_lowercase())
        {
            let header: Vec<_> = current.split(";").collect();
            if header.len() != 2 {
                panic!(
                    "Header does not contain right number of arguments. file: {}",
                    filepath
                );
            }
            if let Some(current_crypt_block) = current_crypt_block.as_mut() {
                if current_crypt_block.algorithm.is_some() || current_crypt_block.nonce.is_some() {
                    panic!("Multiple headers in same crypt block. file: {}", filepath);
                }
                current_crypt_block.algorithm = Some(header[0].to_string());
                current_crypt_block.nonce = Some(header[1].trim().to_string());
            } else {
                panic!(
                    "END CRYPT HEADER without matching start. file: {}",
                    filepath
                );
            }
            current = String::new();
        } else if line.to_lowercase().contains(&END_DELIMITER.to_lowercase()) {
            if current.trim().is_empty() {
                panic!("Crypt block is empty. file: {}", filepath);
            }
            if let Some(mut current_crypt_block) = current_crypt_block {
                current_crypt_block.ciphertext = current.trim_end().to_string();
                blocks.push(Block::Crypt(current_crypt_block));
            } else {
                panic!("END CRYPT without matching start. file: {}", filepath);
            }
            current = String::new();
            current_crypt_block = None;
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }

    if current_crypt_block.is_some() {
        panic!("START CRYPT without matching end. file: {}", filepath);
    }
    if !current.trim().is_empty() {
        blocks.push(Block::Plaintext(current));
    }

    CryptFile { blocks }
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

    let crypt_file = parse_file("file_path", &contents);
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

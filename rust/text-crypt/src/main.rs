use std::fmt;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use base64;
use chacha20poly1305::aead::{Aead, NewAead};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use clap::{App, Arg, SubCommand};
use thiserror::Error;

static START_DELIMITER: &'static str = "---BEGIN CRYPT---";
static END_HEADER_DELIMITER: &'static str = "---END CRYPT HEADER---";
static END_DELIMITER: &'static str = "---END CRYPT---";

const BASE64_CONFIG: base64::Config = base64::STANDARD_NO_PAD;

fn main() {
    let matches = App::new("text-crypt")
        .version("1.0")
        .author("Jonathan Fok kan <jfokkan@gmail.com>")
        .about("Simple text encrypting program")
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(
            SubCommand::with_name("encrypt")
                .aliases(&["e", "enc"])
                .about("Encrypt files containing \"---BEGIN CRYPT---\"")
                .arg(
                    Arg::with_name("password")
                        .env("PASS")
                        .short("p")
                        .required(true)
                        .help("password to be used"),
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("Path to the file to encrypt")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("write")
                        .short("w")
                        .help("Write the result to the input file")
                        .takes_value(false),
                ),
        )
        .subcommand(
            SubCommand::with_name("decrypt")
                .aliases(&["d", "dec"])
                .about("Decrypt files containing \"---BEGIN CRYPT---\"")
                .arg(
                    Arg::with_name("password")
                        .env("PASS")
                        .short("p")
                        .required(true)
                        .help("password to be used"),
                )
                .arg(
                    Arg::with_name("write")
                        .short("w")
                        .help("Write the result to the input file")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("files")
                        .help("Path to the files or directory to encrypt")
                        .required(true)
                        .min_values(1),
                ),
        )
        .get_matches();

    if let Some(enc_matches) = matches.subcommand_matches("encrypt") {
        let password = enc_matches
            .value_of("password")
            .expect("password is required");
        let filename = enc_matches.value_of("INPUT").expect("INPUT is required");
        let contents = fs::read_to_string(filename).expect(&format!("Error reading {}", filename));
        let mut crypt_file = parse_file(filename, &contents);

        let encrypted_crypt_blocks: Vec<_> = crypt_file
            .blocks
            .into_iter()
            .map(|mut block| match block {
                Block::Crypt(ref mut crypt_block) => {
                    if crypt_block.is_encrypted() {
                        block
                    } else {
                        let encrypted_block = encrypt(password, &crypt_block.ciphertext);
                        crypt_block.algorithm = encrypted_block.algorithm;
                        crypt_block.nonce = encrypted_block.nonce;
                        crypt_block.ciphertext = encrypted_block.ciphertext;
                        block
                    }
                }
                Block::Plaintext(_) => block,
            })
            .collect();

        crypt_file.blocks = encrypted_crypt_blocks;

        if enc_matches.is_present("write") {
            let mut file = File::create(filename).expect("create file");
            write!(file, "{}", crypt_file).expect("write file");
        } else {
            println!("{}", crypt_file);
        }
    }

    if let Some(dec_matches) = matches.subcommand_matches("decrypt") {
        let password = dec_matches
            .value_of("password")
            .expect("password is required");
        let files: Vec<_> = dec_matches.values_of("files").unwrap().collect();

        for file_path in files {
            let path = Path::new(&file_path);
            if path.is_dir() {
                todo!()
            } else {
                decrypt_file(dec_matches.is_present("write"), password, path);
            }
        }
    }
}

fn decrypt_file(write: bool, password: &str, filepath: &Path) {
    let contents =
        fs::read_to_string(filepath).expect(&format!("Error reading {}", filepath.display()));
    let mut crypt_file = parse_file(&filepath, &contents);

    let unencrypted_blocks: Vec<_> = crypt_file
        .blocks
        .into_iter()
        .map(|mut block| match block {
            Block::Plaintext(_) => block,
            Block::Crypt(ref mut crypt_block) => {
                if crypt_block.is_encrypted() {
                    let decrypted_text = decrypt(password, &crypt_block);
                    Block::Crypt(CryptBlock::new_unencrypted(&decrypted_text))
                } else {
                    block
                }
            }
        })
        .collect();

    crypt_file.blocks = unencrypted_blocks;
    if write {
        let mut file = File::create(filepath).expect("create file");
        write!(file, "{}", crypt_file).expect("write file");
    } else {
        println!("{}", crypt_file);
    }
}

fn encrypt(password: &str, contents: &str) -> CryptBlock {
    let key = Key::from_slice(password.as_bytes()); // 32-bytes
    let cipher = ChaCha20Poly1305::new(key);

    let nonce_str = "unique nonce";
    let nonce = Nonce::from_slice(nonce_str.as_bytes()); // 12-bytes; unique per message

    let ciphertext_bytes = cipher
        .encrypt(nonce, contents.as_bytes().as_ref())
        .expect("encryption failure!"); // NOTE: handle this error to avoid panics!

    let ciphertext = base64::encode_config(ciphertext_bytes, BASE64_CONFIG);
    CryptBlock {
        algorithm: Some("ChaCha20Poly1305".to_string()),
        nonce: Some(nonce_str.to_string()),
        ciphertext,
    }
}

fn decrypt(password: &str, encrypted: &CryptBlock) -> String {
    let key = Key::from_slice(password.as_bytes()); // 32-bytes
    let cipher = ChaCha20Poly1305::new(key);

    let nonce = Nonce::from_slice(encrypted.nonce.as_ref().expect("missing nonce").as_bytes()); // 12-bytes; unique per message

    let ciphertext_bytes =
        base64::decode_config(&encrypted.ciphertext, BASE64_CONFIG).expect("failed base64 decode");

    let plaintext = cipher
        .decrypt(nonce, ciphertext_bytes.as_ref())
        .expect("decryption failure!"); // NOTE: handle this error to avoid panics!

    String::from_utf8(plaintext).expect("failed decoding plaintext bytes to utf8")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Block {
    Plaintext(String),
    Crypt(CryptBlock),
}

struct CryptFile {
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
struct CryptBlock {
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

fn parse_file<T: AsRef<Path>>(filepath: T, contents: &str) -> CryptFile {
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

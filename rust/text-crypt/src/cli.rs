use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use clap::{App, Arg, SubCommand};
use thiserror::Error;
use walkdir::WalkDir;

use crate::{
    crypto::{decrypt, encrypt},
    parse::parse_file,
    Block, CryptBlock, START_DELIMITER,
};

pub fn run() {
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
        let should_write = enc_matches.is_present("write");

        encrypt_file(password, should_write, filename, &contents);
    }

    if let Some(dec_matches) = matches.subcommand_matches("decrypt") {
        let password = dec_matches
            .value_of("password")
            .expect("password is required");
        let files: Vec<_> = dec_matches.values_of("files").unwrap().collect();
        let should_write = dec_matches.is_present("write");
        let should_print_filename = files.len() > 1;

        for file_path in files {
            let path = Path::new(&file_path);
            if path.is_dir() {
                for entry in WalkDir::new(path) {
                    let dir_entry = entry.expect("read entry");
                    if dir_entry.path().is_file() {
                        decrypt_file(should_write, password, dir_entry.path(), true);
                    }
                }
            } else {
                decrypt_file(should_write, password, path, should_print_filename);
            }
        }
    }
}

fn encrypt_file(password: &str, should_write: bool, filename: &str, contents: &str) {
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

    if should_write {
        let mut file = File::create(filename).expect("create file");
        write!(file, "{}", crypt_file).expect("write file");
    } else {
        println!("{}", crypt_file);
    }
}

fn decrypt_file(write: bool, password: &str, filepath: &Path, should_print_filename: bool) {
    let contents =
        fs::read_to_string(filepath).expect(&format!("Error reading {}", filepath.display()));

    if !contents.contains(START_DELIMITER) {
        return;
    }
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
        if should_print_filename {
            println!("{}", filepath.display());
        }
        println!("{}", crypt_file);
    }
}

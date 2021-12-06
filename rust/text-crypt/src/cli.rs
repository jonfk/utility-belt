use std::path::Path;
use std::{fs, path::PathBuf};

use clap::{App, Arg, SubCommand};
use walkdir::{DirEntry, WalkDir};

use crate::CryptFile;
use crate::{Block, CheckError, CheckErrors};

mod decrypt;
mod encrypt;

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
                        ,
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
        .subcommand(
            SubCommand::with_name("check")
                .aliases(&["c"])
                .about("Check that no files containing \"---BEGIN CRYPT---\" are unencrypted")
                .arg(Arg::with_name("files").help("Path to the files or directory to encrypt. Defaults to current directory if none is supplied")),
        )
        .get_matches();

    if let Some(enc_matches) = matches.subcommand_matches("encrypt") {
        let password = enc_matches
            .value_of("password")
            .expect("password is required");
        let paths: Vec<_> = enc_matches.values_of("INPUT").unwrap().collect();
        let write_file = enc_matches.is_present("write");

        encrypt::encrypt_cmd(password, write_file, paths).expect("encrypt");
    } else if let Some(dec_matches) = matches.subcommand_matches("decrypt") {
        let password = dec_matches
            .value_of("password")
            .expect("password is required");
        let paths: Vec<_> = dec_matches.values_of("files").unwrap().collect();
        let write_file = dec_matches.is_present("write");

        decrypt::decrypt_cmd(write_file, password, paths);
    } else if let Some(check_matches) = matches.subcommand_matches("check") {
        let files: Vec<_> = check_matches
            .values_of("files")
            .unwrap_or_default()
            .collect();
        check_cmd(files).expect("check_files");
    }
}

fn check_cmd(files: Vec<&str>) -> Result<(), CheckErrors> {
    let files = if files.is_empty() {
        vec![std::env::current_dir().map_err(|e| CheckErrors {
            errors: vec![CheckError::ReadFile(
                "current working directory".to_string(),
                e,
            )],
        })?]
    } else {
        files.into_iter().map(|s| PathBuf::from(s)).collect()
    };
    let errors: Vec<CheckError> = files
        .iter()
        .flat_map(|input_path| {
            let path = Path::new(&input_path);
            if path.is_dir() {
                walk_dir(path)
                    .map(|entry_res| {
                        let entry = entry_res
                            .map_err(|e| CheckError::WalkDir(format!("{}", path.display()), e))?;

                        if entry.path().is_file() {
                            check_file(entry.path())
                        } else {
                            Ok(())
                        }
                    })
                    .collect()
            } else {
                vec![check_file(path)]
            }
        })
        .filter_map(|result| match result {
            Ok(_) => None,
            Err(err) => Some(err),
        })
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CheckErrors { errors })
    }
}

fn check_file(file_path: &Path) -> Result<(), CheckError> {
    let contents = fs::read_to_string(file_path)
        .map_err(|e| CheckError::ReadFile(format!("{}", file_path.display()), e))?;

    if !CryptFile::is_crypt_file(&contents) {
        return Ok(());
    }
    let crypt_file = CryptFile::from_str(&contents)
        .map_err(|e| CheckError::ParseCryptFile(format!("{}", file_path.display()), e))?;
    if crypt_file
        .blocks
        .into_iter()
        .filter(|block| match block {
            Block::Plaintext(_) => false,
            Block::Crypt(crypt_block) => !crypt_block.is_encrypted(),
        })
        .count()
        > 0
    {
        return Err(CheckError::UnencryptedFile(format!(
            "{}",
            file_path.display()
        )));
    }
    Ok(())
}

fn walk_dir<P: AsRef<Path>>(
    path: P,
) -> walkdir::FilterEntry<walkdir::IntoIter, fn(&DirEntry) -> bool> {
    WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| !is_hidden_or_binary(e))
}

fn is_hidden_or_binary(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| (!s.eq(".") && !s.eq("..") && s.starts_with(".")) || s.ends_with(".gpg"))
        .unwrap_or(false)
}

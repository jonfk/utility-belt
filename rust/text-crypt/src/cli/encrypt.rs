use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    crypto::encrypt,
    error::{EncryptError, EncryptErrors},
    Block, CryptFile,
};

use super::walk_dir;

pub(crate) fn encrypt_cmd(
    verbose: bool,
    password: &str,
    write_file: bool,
    paths: Vec<&str>,
) -> Result<(), EncryptErrors> {
    let paths = if paths.is_empty() {
        vec![std::env::current_dir().map_err(|e| {
            EncryptErrors::new(vec![EncryptError::ReadFile(
                "current working directory".to_string(),
                e,
            )])
        })?]
    } else {
        paths.into_iter().map(|s| PathBuf::from(s)).collect()
    };
    let print_filenames = paths.len() > 1;
    let errors: Vec<_> = paths
        .into_iter()
        .flat_map(|path| {
            if path.is_dir() {
                encrypt_dir(verbose, password, write_file, &path)
            } else {
                vec![encrypt_file(
                    verbose,
                    password,
                    write_file,
                    path,
                    print_filenames,
                )]
            }
        })
        .filter_map(|res| match res {
            Ok(_) => None,
            Err(e) => Some(e),
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(EncryptErrors::new(errors))
    }
}

fn encrypt_dir(
    verbose: bool,
    password: &str,
    write_file: bool,
    path: &Path,
) -> Vec<Result<(), EncryptError>> {
    walk_dir(path)
        .map(|direntry| {
            let entry =
                direntry.map_err(|e| EncryptError::WalkDir(format!("{}", path.display()), e))?;
            let entry_path = entry.path();
            if entry_path.is_file() {
                encrypt_file(verbose, password, write_file, entry_path, true)?;
            }
            Ok(())
        })
        .collect()
}

fn encrypt_file<P: AsRef<Path>>(
    verbose: bool,
    password: &str,
    write_file: bool,
    path: P,
    print_filename: bool,
) -> Result<(), EncryptError> {
    let filename = format!("{}", path.as_ref().display());
    let contents = fs::read_to_string(path.as_ref())
        .map_err(|e| EncryptError::ReadFile(filename.clone(), e))?;

    if !CryptFile::is_crypt_file(&contents) {
        if verbose {
            eprintln!("Skipping encrypting {} since not a Crypt File", filename);
        }
        return Ok(());
    }
    let mut crypt_file = CryptFile::from_str(&contents)
        .map_err(|e| EncryptError::ParseCryptFile(filename.clone(), e))?;

    let encrypted_crypt_blocks: Result<Vec<_>, EncryptError> = crypt_file
        .blocks
        .into_iter()
        .map(|mut block| match block {
            Block::Crypt(ref mut crypt_block) => {
                if crypt_block.is_encrypted() {
                    Ok(block)
                } else {
                    let encrypted_block = encrypt(password, &crypt_block.ciphertext)?;
                    crypt_block.algorithm = encrypted_block.algorithm;
                    crypt_block.nonce = encrypted_block.nonce;
                    crypt_block.ciphertext = encrypted_block.ciphertext;
                    Ok(block)
                }
            }
            Block::Plaintext(_) => Ok(block),
        })
        .collect();

    crypt_file.blocks = encrypted_crypt_blocks?;

    if write_file {
        let mut file = File::create(path.as_ref())
            .map_err(|e| EncryptError::WriteFile(filename.to_string(), e))?;
        write!(file, "{}", crypt_file)
            .map_err(|e| EncryptError::WriteFile(filename.to_string(), e))?;
    } else {
        if print_filename {
            println!("{}", filename);
        }
        println!("{}", crypt_file);
    }
    Ok(())
}

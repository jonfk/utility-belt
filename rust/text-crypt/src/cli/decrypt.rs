use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    crypto::decrypt,
    error::{DecryptError, DecryptErrors},
    Block, CryptFile,
};

use super::walk_dir;

pub(crate) fn decrypt_cmd(
    verbose: bool,
    write_file: bool,
    password: &str,
    paths: Vec<&str>,
) -> Result<(), DecryptErrors> {
    let paths = if paths.is_empty() {
        vec![std::env::current_dir().map_err(|e| {
            DecryptErrors::new(vec![DecryptError::ReadFile(
                "current working directory".to_string(),
                e,
            )])
        })?]
    } else {
        paths.into_iter().map(|s| PathBuf::from(s)).collect()
    };
    let should_print_filename = paths.len() > 1 || paths[0].is_dir();

    let errors: Vec<_> = paths
        .into_iter()
        .flat_map(|path| {
            if path.is_dir() {
                decrypt_dir(verbose, write_file, password, &path)
            } else {
                vec![decrypt_file(
                    verbose,
                    write_file,
                    password,
                    &path,
                    should_print_filename,
                )]
            }
        })
        .filter_map(|res| match res {
            Ok(_) => None,
            Err(e) => Some(e),
        })
        .collect();
    if errors.len() > 0 {
        Err(DecryptErrors::new(errors))
    } else {
        Ok(())
    }
}

fn decrypt_dir(
    verbose: bool,
    write_file: bool,
    password: &str,
    path: &Path,
) -> Vec<Result<(), DecryptError>> {
    walk_dir(path)
        .map(|dir_entry| {
            let entry =
                dir_entry.map_err(|e| DecryptError::WalkDir(format!("{}", path.display()), e))?;
            if entry.path().is_file() {
                decrypt_file(verbose, write_file, password, entry.path(), true)?;
            }
            Ok(())
        })
        .collect()
}

fn decrypt_file(
    verbose: bool,
    write: bool,
    password: &str,
    filepath: &Path,
    should_print_filename: bool,
) -> Result<(), DecryptError> {
    let filename = format!("{}", filepath.display());
    let contents =
        fs::read_to_string(filepath).map_err(|e| DecryptError::ReadFile(filename.clone(), e))?;

    if !CryptFile::is_crypt_file(&contents) {
        if verbose {
            eprintln!("Skipping decrypting {} since not a Crypt File", filename);
        }
        return Ok(());
    }
    let mut crypt_file = CryptFile::from_str(&contents)
        .map_err(|e| DecryptError::ParseCryptFile(filename.clone(), e))?;

    let unencrypted_blocks: Result<Vec<_>, DecryptError> = crypt_file
        .blocks
        .into_iter()
        .map(|mut block| match block {
            Block::Plaintext(_) => Ok(block),
            Block::UnencryptedCryptBlock(_) => Ok(block),
            Block::EncryptedCryptBlock(ref mut crypt_block) => {
                let decrypted_text = decrypt(password, &crypt_block)?;
                Ok(Block::UnencryptedCryptBlock(decrypted_text))
            }
        })
        .collect();

    crypt_file.blocks = unencrypted_blocks?;
    if write {
        let mut file =
            File::create(filepath).map_err(|e| DecryptError::WriteFile(filename.clone(), e))?;
        write!(file, "{}", crypt_file).map_err(|e| DecryptError::WriteFile(filename.clone(), e))?;
    } else {
        if should_print_filename {
            println!("{}", filepath.display());
        }
        println!("{}", crypt_file);
    }
    Ok(())
}

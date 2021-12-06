use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use crate::{crypto::encrypt, Block, CryptFile, EncryptError, EncryptErrors};

use super::walk_dir;

pub(crate) fn encrypt_cmd(
    password: &str,
    write_file: bool,
    paths: Vec<&str>,
) -> Result<(), EncryptErrors> {
    let paths = if paths.is_empty() {
        vec![std::env::current_dir().map_err(|e| EncryptErrors {
            errors: vec![EncryptError::ReadFile(
                "current working directory".to_string(),
                e,
            )],
        })?]
    } else {
        paths.into_iter().map(|s| PathBuf::from(s)).collect()
    };
    let errors: Vec<_> = paths
        .into_iter()
        .flat_map(|path| {
            if path.is_dir() {
                encrypt_dir(password, write_file, &path)
            } else {
                vec![encrypt_file(password, write_file, path)]
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
        Err(EncryptErrors { errors })
    }
}

fn encrypt_dir(password: &str, write_file: bool, path: &Path) -> Vec<Result<(), EncryptError>> {
    walk_dir(path)
        .map(|direntry| {
            let entry =
                direntry.map_err(|e| EncryptError::WalkDir(format!("{}", path.display()), e))?;
            let entry_path = entry.path();
            if entry_path.is_file() {
                encrypt_file(password, write_file, entry_path)?;
            }
            Ok(())
        })
        .collect()
}

fn encrypt_file<P: AsRef<Path>>(
    password: &str,
    should_write: bool,
    path: P,
) -> Result<(), EncryptError> {
    let filename = format!("{}", path.as_ref().display());
    let contents = fs::read_to_string(path.as_ref())
        .map_err(|e| EncryptError::ReadFile(filename.clone(), e))?;

    if !CryptFile::is_crypt_file(&contents) {
        return Ok(());
    }
    let mut crypt_file = CryptFile::from_str(&contents)
        .map_err(|e| EncryptError::ParseCryptFile(filename.clone(), e))?;

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
        let mut file = File::create(path.as_ref())
            .map_err(|e| EncryptError::WriteFile(filename.to_string(), e))?;
        write!(file, "{}", crypt_file)
            .map_err(|e| EncryptError::WriteFile(filename.to_string(), e))?;
    } else {
        println!("{}", crypt_file);
    }
    Ok(())
}

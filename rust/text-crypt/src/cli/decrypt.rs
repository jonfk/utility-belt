use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use crate::{crypto::decrypt, Block, CryptBlock, CryptFile};

use super::walk_dir;

pub(crate) fn decrypt_cmd(write_file: bool, password: &str, paths: Vec<&str>) {
    let should_print_filename = paths.len() > 1;

    for file_path in paths {
        let path = Path::new(&file_path);
        if path.is_dir() {
            for entry in walk_dir(path) {
                let dir_entry = entry.expect("read entry");
                if dir_entry.path().is_file() {
                    decrypt_file(write_file, password, dir_entry.path(), true);
                }
            }
        } else {
            decrypt_file(write_file, password, path, should_print_filename);
        }
    }
}

fn decrypt_file(write: bool, password: &str, filepath: &Path, should_print_filename: bool) {
    let contents =
        fs::read_to_string(filepath).expect(&format!("Error reading {}", filepath.display()));

    if CryptFile::is_crypt_file(&contents) {
        return;
    }
    let mut crypt_file = CryptFile::from_str(&contents).expect("parse failed");

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

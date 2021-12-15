use std::path::Path;
use std::{fs, path::PathBuf};

use crate::error::{CheckError, CheckErrors};
use crate::CryptFile;

use super::walk_dir;

pub fn check_cmd(files: Vec<&str>) -> Result<(), CheckErrors> {
    let files = if files.is_empty() {
        vec![std::env::current_dir().map_err(|e| {
            CheckErrors::new(vec![CheckError::ReadFile(
                "current working directory".to_string(),
                e,
            )])
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
        Err(CheckErrors::new(errors))
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
    if crypt_file.has_unencrypted_crypt_blocks() {
        return Err(CheckError::UnencryptedFile(format!(
            "{}",
            file_path.display()
        )));
    }
    Ok(())
}

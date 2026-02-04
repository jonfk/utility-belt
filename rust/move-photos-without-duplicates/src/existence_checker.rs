use crate::db::Database;
use crate::progress::ProgressManager;
use crate::scanner::FileScanner;
use crate::types::AppError;
use camino::Utf8PathBuf;
use error_stack::{Report, Result, ResultExt};
use indicatif::MultiProgress;
use std::fs;
use walkdir::WalkDir;

pub struct ExistenceChecker {
    scanner: FileScanner,
}

pub struct CheckExistResult {
    pub files_checked: usize,
    pub matches_found: usize,
    pub errors: usize,
}

impl ExistenceChecker {
    pub fn new(batch_size: usize) -> Self {
        Self {
            scanner: FileScanner::new(batch_size.max(1)),
        }
    }

    pub async fn check_path(
        &self,
        path: &Utf8PathBuf,
        db: &Database,
        multi: &MultiProgress,
    ) -> Result<CheckExistResult, AppError> {
        if !path.exists() {
            return Err(Report::new(AppError::Validation)
                .attach_printable(format!("Path does not exist: {}", path)));
        }

        let files = self.collect_files(path, multi)?;

        if files.is_empty() {
            println!("No files found to check under {}", path);
            return Ok(CheckExistResult {
                files_checked: 0,
                matches_found: 0,
                errors: 0,
            });
        }

        let process_pb = ProgressManager::create_process_progress(multi, files.len() as u64);
        process_pb.set_message("Checking files against database...");

        let mut result = CheckExistResult {
            files_checked: 0,
            matches_found: 0,
            errors: 0,
        };

        for file_path in files {
            process_pb.inc(1);
            result.files_checked += 1;

            match self.process_single_file(&file_path, db).await {
                Ok(Some(matches)) => {
                    result.matches_found += 1;
                    println!("MATCH: {}", file_path);
                    for stored in matches {
                        println!("  stored: {}", stored);
                    }
                }
                Ok(None) => {
                    // Intentionally quiet on misses per user preference
                }
                Err(e) => {
                    result.errors += 1;
                    eprintln!("ERROR: Failed to check {}", file_path);
                    eprintln!("  Details: {:?}", e);
                }
            }
        }

        process_pb.finish_with_message("✓ Check complete");

        if result.files_checked > 0
            && result.errors == 0
            && result.matches_found == result.files_checked
        {
            println!("\nAll files are already present in the hash database.");
        }

        Ok(result)
    }

    fn collect_files(
        &self,
        path: &Utf8PathBuf,
        multi: &MultiProgress,
    ) -> Result<Vec<Utf8PathBuf>, AppError> {
        if path.is_file() {
            return Ok(vec![path.clone()]);
        }

        if !path.is_dir() {
            return Err(Report::new(AppError::Validation)
                .attach_printable(format!("Path is neither file nor directory: {}", path)));
        }

        let scan_pb =
            ProgressManager::create_scan_progress(multi, &format!("Scanning directory: {}", path));

        let mut files = Vec::new();
        for entry in WalkDir::new(path) {
            scan_pb.tick();
            match entry {
                Ok(dir_entry) => {
                    if dir_entry.file_type().is_file() {
                        match Utf8PathBuf::from_path_buf(dir_entry.path().to_path_buf()) {
                            Ok(p) => files.push(p),
                            Err(p) => {
                                eprintln!("ERROR: Non-UTF8 path encountered: {:?}", p);
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("ERROR: Directory traversal failed");
                    if let Some(p) = err.path() {
                        eprintln!("  Path: {:?}", p);
                    }
                    if let Some(io_err) = err.io_error() {
                        eprintln!("  IO Error: {}", io_err);
                    }
                }
            }
        }

        scan_pb.finish_with_message(format!("✓ Found {} files", files.len()));
        Ok(files)
    }

    async fn process_single_file(
        &self,
        file_path: &Utf8PathBuf,
        db: &Database,
    ) -> Result<Option<Vec<Utf8PathBuf>>, AppError> {
        // Ensure path is a file
        let metadata = fs::metadata(file_path)
            .change_context(AppError::IO)
            .attach_printable_lazy(|| format!("Failed to stat: {}", file_path))?;

        if !metadata.is_file() {
            return Ok(None);
        }

        let hash = self
            .scanner
            .calculate_file_hash(file_path)
            .change_context(AppError::FileProcessing)?;

        let matches = db
            .get_files_with_hash(&hash)
            .await
            .change_context(AppError::DB)
            .attach_printable_lazy(|| format!("hash={}", hash))?;

        let mut existing_matches = Vec::with_capacity(matches.len());
        for stored_path in matches {
            if stored_path.exists() {
                existing_matches.push(stored_path);
            } else {
                eprintln!(
                    "INFO: skipping stale database entry (file missing): {}",
                    stored_path
                );
            }
        }

        if existing_matches.is_empty() {
            Ok(None)
        } else {
            Ok(Some(existing_matches))
        }
    }
}

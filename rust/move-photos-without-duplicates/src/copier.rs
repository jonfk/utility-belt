use crate::db::Database;
use crate::progress::ProgressManager;
use crate::scanner::FileScanner;
use crate::types::{AppError, CopyOperationResult, SingleCopyResult};
use camino::Utf8PathBuf;
use error_stack::{Result, ResultExt};
use indicatif::MultiProgress;
use std::fs;
use walkdir::WalkDir;

pub struct FileCopier {
    scanner: FileScanner,
}

impl FileCopier {
    pub fn new() -> Self {
        Self {
            scanner: FileScanner::new(1), // Single file processing for copying
        }
    }

    pub fn validate_copy_command(
        source: &Utf8PathBuf,
        target: &Utf8PathBuf,
    ) -> Result<(), AppError> {
        // Check that both paths are directories
        if !source.is_dir() {
            return Err(
                error_stack::Report::new(AppError::Validation).attach_printable(format!(
                    "Source directory does not exist or is not a directory: {}",
                    source
                )),
            );
        }

        if !target.is_dir() {
            return Err(
                error_stack::Report::new(AppError::Validation).attach_printable(format!(
                    "Target directory does not exist or is not a directory: {}",
                    target
                )),
            );
        }

        println!("Validation passed:");
        println!("  Source directory: {}", source);
        println!("  Target directory: {}", target);

        // Print warning about target directory needing to be hashed
        println!(
            "\n⚠️  WARNING: The target directory should be hashed before running copy operations"
        );
        println!("   to ensure effective duplicate detection. Run:");
        println!("   {} hash {}", env!("CARGO_PKG_NAME"), target);

        Ok(())
    }

    pub async fn copy_files_without_duplicates(
        &self,
        dir_a: &Utf8PathBuf,
        dir_b: &Utf8PathBuf,
        db: &Database,
        multi: &MultiProgress,
        dry_run: bool,
    ) -> Result<CopyOperationResult, AppError> {
        println!("Starting copy operation from {} to {}", dir_a, dir_b);
        if dry_run {
            println!("DRY RUN MODE: No files will actually be copied");
        }

        let mut result = CopyOperationResult::new();

        // Phase 1: Directory scanning with spinner
        let scan_pb = ProgressManager::create_scan_progress(
            multi,
            &format!("Scanning source directory: {}", dir_a),
        );

        // Collect all files in directory A
        let mut file_paths = Vec::new();
        let mut traversal_errors = 0;

        for entry in WalkDir::new(dir_a) {
            scan_pb.tick();
            match entry {
                Ok(dir_entry) => {
                    if dir_entry.file_type().is_file() {
                        match Utf8PathBuf::from_path_buf(dir_entry.path().to_path_buf()) {
                            Ok(utf8_path) => {
                                file_paths.push(utf8_path);
                                scan_pb.set_message(format!(
                                    "Scanning... found {} files",
                                    file_paths.len()
                                ));
                            }
                            Err(path_buf) => {
                                eprintln!("ERROR: Non-UTF8 path encountered: {:?}", path_buf);
                                traversal_errors += 1;
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("ERROR: Directory traversal failed for source");
                    if let Some(path) = err.path() {
                        eprintln!("  Path: {:?}", path);
                    }
                    eprintln!("  Error: {}", err);
                    traversal_errors += 1;
                }
            }
        }

        if traversal_errors > 0 {
            eprintln!(
                "WARNING: {} errors during source directory traversal",
                traversal_errors
            );
        }

        scan_pb.finish_with_message(format!("✓ Found {} files to copy", file_paths.len()));

        // Phase 2: File processing with progress bar
        let process_pb = ProgressManager::create_copy_progress(multi, file_paths.len() as u64);
        process_pb.set_message("Processing files...");

        for file_path in file_paths {
            match self
                .process_single_file(&file_path, dir_a, dir_b, db, dry_run)
                .await
            {
                Ok(SingleCopyResult::Copied) => {
                    result.copied += 1;
                    process_pb.set_message(format!(
                        "Copied: {} files, {} skipped, {} errors",
                        result.copied, result.skipped, result.errors
                    ));
                    println!("COPIED: {}", file_path);
                }
                Ok(SingleCopyResult::Skipped(reason)) => {
                    result.skipped += 1;
                    process_pb.set_message(format!(
                        "Copied: {} files, {} skipped, {} errors",
                        result.copied, result.skipped, result.errors
                    ));
                    println!("SKIPPED: {} ({})", file_path, reason);
                }
                Err(e) => {
                    result.errors += 1;
                    process_pb.set_message(format!(
                        "Copied: {} files, {} skipped, {} errors",
                        result.copied, result.skipped, result.errors
                    ));
                    eprintln!("ERROR: Failed to process {}", file_path);
                    eprintln!("  Details: {:?}", e);
                }
            }
            process_pb.inc(1);
        }

        process_pb.finish_with_message(format!(
            "✓ Copy complete: {} copied, {} skipped, {} errors",
            result.copied, result.skipped, result.errors
        ));

        // Print summary
        println!("\nCOPY OPERATION SUMMARY:");
        println!("  Files copied: {}", result.copied);
        println!("  Files skipped: {}", result.skipped);
        println!("  Errors: {}", result.errors);

        if dry_run {
            println!("  (This was a dry run - no files were actually copied)");
        }

        Ok(result)
    }

    async fn process_single_file(
        &self,
        file_path: &Utf8PathBuf,
        source_base: &Utf8PathBuf,
        target_base: &Utf8PathBuf,
        db: &Database,
        dry_run: bool,
    ) -> Result<SingleCopyResult, AppError> {
        // Calculate relative path from source base
        let relative_path = file_path
            .strip_prefix(source_base)
            .change_context(AppError::Copy)
            .attach_printable_lazy(|| {
                format!("Failed to calculate relative path for: {}", file_path)
            })?;

        // Calculate target path
        let target_path = target_base.join(relative_path);

        // Check if target file already exists
        if target_path.exists() {
            return Ok(SingleCopyResult::Skipped(
                "target file already exists".to_string(),
            ));
        }

        // Get file metadata and hash
        let metadata = fs::metadata(file_path)
            .change_context(AppError::IO)
            .attach_printable_lazy(|| format!("Failed to get metadata for: {}", file_path))?;

        let last_modified = metadata
            .modified()
            .change_context(AppError::IO)
            .attach_printable_lazy(|| {
                format!("Failed to get modification time for: {}", file_path)
            })?;

        let file_hash = self.scanner.calculate_file_hash(file_path)?;

        // Check if this hash already exists in the database (duplicate)
        let existing_files = db
            .get_files_with_hash(&file_hash)
            .await
            .change_context(AppError::DB)
            .attach_printable_lazy(|| format!("file={file_path}"))?;

        if !existing_files.is_empty() {
            // Track this as a duplicate file
            let original_path = &existing_files[0]; // Use first match as original
            let file_size = metadata.len();

            if let Err(e) = db
                .track_duplicate_file(file_path, original_path, &file_hash, file_size)
                .await
            {
                eprintln!(
                    "WARNING: Failed to track duplicate file {}: {:?}",
                    file_path, e
                );
            }

            return Ok(SingleCopyResult::Skipped(format!(
                "duplicate content (matches {})",
                original_path
            )));
        }

        if dry_run {
            return Ok(SingleCopyResult::Copied);
        }

        // Create target directory if it doesn't exist
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .change_context(AppError::Copy)
                .attach_printable_lazy(|| {
                    format!("Failed to create target directory: {}", parent)
                })?;
        }

        // Copy the file
        fs::copy(file_path, &target_path)
            .change_context(AppError::Copy)
            .attach_printable_lazy(|| format!("Failed to copy {} to {}", file_path, target_path))?;

        // Record the copied file atomically (both hash storage and copy tracking)
        let file_size = metadata.len();
        if let Err(e) = db
            .record_copied_file(
                file_path,
                &target_path,
                &file_hash,
                file_size,
                last_modified,
            )
            .await
        {
            eprintln!(
                "WARNING: Failed to record copied file {} -> {}: {:?}",
                file_path, target_path, e
            );
        }

        Ok(SingleCopyResult::Copied)
    }
}

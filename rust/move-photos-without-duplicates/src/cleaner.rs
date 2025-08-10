use crate::db::{CopiedFileRecord, Database, DuplicateFileRecord};
use crate::progress::ProgressManager;
use crate::scanner::FileScanner;
use crate::types::{AppError, CleanupOperationResult, SingleCleanupResult};
use error_stack::{Result, ResultExt};
use indicatif::MultiProgress;
use std::fs;

pub struct FileCleaner;

impl FileCleaner {
    pub fn new() -> Self {
        Self
    }

    pub async fn cleanup_tracked_files(
        &self,
        db: &Database,
        scanner: &FileScanner,
        multi: &MultiProgress,
        dry_run: bool,
    ) -> Result<CleanupOperationResult, AppError> {
        println!("Starting cleanup of tracked files");
        if dry_run {
            println!("DRY RUN MODE: No files will actually be deleted");
        }

        let mut result = CleanupOperationResult::new();

        // Phase 1: Get all tracked files
        let scan_pb = ProgressManager::create_scan_progress(multi, "Loading tracked files from database...");

        let copied_files = db
            .get_copied_files()
            .await
            .change_context(AppError::Cleanup)
            .attach_printable("Failed to retrieve copied files")?;

        let duplicate_files = db
            .get_duplicate_files()
            .await
            .change_context(AppError::Cleanup)
            .attach_printable("Failed to retrieve duplicate files")?;

        scan_pb.finish_with_message(format!(
            "✓ Found {} copied files and {} duplicate files to process",
            copied_files.len(),
            duplicate_files.len()
        ));

        let total_files = copied_files.len() + duplicate_files.len();
        if total_files == 0 {
            println!("No tracked files found for cleanup");
            return Ok(result);
        }

        // Phase 2: Process files
        let process_pb = ProgressManager::create_cleanup_progress(multi, total_files as u64);
        process_pb.set_message("Processing copied files...");

        println!("\n=== PROCESSING COPIED FILES ===");
        for copied_file in copied_files {
            match self.cleanup_copied_file(&copied_file, db, scanner, dry_run).await {
                Ok(SingleCleanupResult::Deleted) => {
                    result.deleted += 1;
                    println!(
                        "DELETED: {} (target: {})",
                        copied_file.source_path, copied_file.target_path
                    );
                }
                Ok(SingleCleanupResult::Skipped(reason)) => {
                    result.skipped += 1;
                    println!("SKIPPED: {} ({})", copied_file.source_path, reason);
                }
                Err(e) => {
                    result.errors += 1;
                    eprintln!(
                        "ERROR: Failed to cleanup {}: {:?}",
                        copied_file.source_path, e
                    );
                }
            }
            process_pb.inc(1);
            process_pb.set_message(format!(
                "Cleaned: {} deleted, {} skipped, {} errors",
                result.deleted, result.skipped, result.errors
            ));
        }

        println!("\n=== PROCESSING DUPLICATE FILES ===");
        for duplicate_file in duplicate_files {
            match self.cleanup_duplicate_file(&duplicate_file, db, scanner, dry_run).await {
                Ok(SingleCleanupResult::Deleted) => {
                    result.deleted += 1;
                    println!(
                        "DELETED: {} (duplicate of: {})",
                        duplicate_file.duplicate_path, duplicate_file.original_path
                    );
                }
                Ok(SingleCleanupResult::Skipped(reason)) => {
                    result.skipped += 1;
                    println!("SKIPPED: {} ({})", duplicate_file.duplicate_path, reason);
                }
                Err(e) => {
                    result.errors += 1;
                    eprintln!(
                        "ERROR: Failed to cleanup {}: {:?}",
                        duplicate_file.duplicate_path, e
                    );
                }
            }
            process_pb.inc(1);
            process_pb.set_message(format!(
                "Cleaned: {} deleted, {} skipped, {} errors",
                result.deleted, result.skipped, result.errors
            ));
        }

        process_pb.finish_with_message(format!(
            "✓ Cleanup complete: {} deleted, {} skipped, {} errors",
            result.deleted, result.skipped, result.errors
        ));

        // Print summary
        println!("\nCLEANUP OPERATION SUMMARY:");
        println!("  Files deleted: {}", result.deleted);
        println!("  Files skipped: {}", result.skipped);
        println!("  Errors: {}", result.errors);

        if dry_run {
            println!("  (This was a dry run - no files were actually deleted)");
        }

        Ok(result)
    }

    pub async fn show_status(
        &self,
        db: &Database,
        multi: &MultiProgress,
        detailed: bool,
    ) -> Result<(), AppError> {
        println!("Retrieving status of files tracked for cleanup...");

        // Get tracked files from database with spinner
        let scan_pb = ProgressManager::create_scan_progress(multi, "Loading tracked files from database...");

        let copied_files = db
            .get_copied_files()
            .await
            .change_context(AppError::DB)
            .attach_printable("Failed to retrieve copied files")?;

        let duplicate_files = db
            .get_duplicate_files()
            .await
            .change_context(AppError::DB)
            .attach_printable("Failed to retrieve duplicate files")?;

        scan_pb.finish_with_message("✓ Status retrieved");

        // Calculate statistics
        let copied_count = copied_files.len();
        let duplicate_count = duplicate_files.len();
        let total_files = copied_count + duplicate_count;

        let copied_total_size: u64 = copied_files.iter().map(|f| f.file_size).sum();
        let duplicate_total_size: u64 = duplicate_files.iter().map(|f| f.file_size).sum();
        let total_size = copied_total_size + duplicate_total_size;

        // Display summary
        println!("\n=== CLEANUP STATUS SUMMARY ===");
        println!("Files that would be deleted by cleanup command:");
        println!("  Copied files (source files): {}", copied_count);
        println!("  Duplicate files: {}", duplicate_count);
        println!("  Total files: {}", total_files);

        if total_size > 0 {
            println!(
                "  Total size: {} bytes ({:.2} MB)",
                total_size,
                total_size as f64 / (1024.0 * 1024.0)
            );
        }

        if total_files == 0 {
            println!("\n✓ No files are currently tracked for cleanup");
            return Ok(());
        }

        // Show detailed listing if requested
        if detailed {
            if copied_count > 0 {
                println!("\n=== COPIED FILES (source files that would be deleted) ===");
                for copied_file in &copied_files {
                    println!(
                        "  {} -> {}",
                        copied_file.source_path, copied_file.target_path
                    );
                }
            }

            if duplicate_count > 0 {
                println!("\n=== DUPLICATE FILES (that would be deleted) ===");
                for duplicate_file in &duplicate_files {
                    println!(
                        "  {} (duplicate of: {})",
                        duplicate_file.duplicate_path, duplicate_file.original_path
                    );
                }
            }
        } else if total_files > 0 {
            println!("\nUse --detailed to see the full list of file paths that would be deleted.");
        }

        Ok(())
    }

    async fn cleanup_copied_file(
        &self,
        copied_file: &CopiedFileRecord,
        db: &Database,
        scanner: &FileScanner,
        dry_run: bool,
    ) -> Result<SingleCleanupResult, AppError> {
        // Check if target file still exists to verify the copy was successful
        if !copied_file.target_path.exists() {
            return Ok(SingleCleanupResult::Skipped(
                "target file no longer exists - copy may have failed".to_string(),
            ));
        }

        // Verify target file integrity by checking its hash matches the stored hash
        if !dry_run {
            let target_hash = scanner.calculate_file_hash(&copied_file.target_path)
                .change_context(AppError::Cleanup)
                .attach_printable_lazy(|| {
                    format!("Failed to calculate hash for target file: {}", copied_file.target_path)
                })?;
            
            if target_hash != copied_file.hash {
                return Ok(SingleCleanupResult::Skipped(
                    "target hash mismatch; refusing to delete source".to_string(),
                ));
            }
        }

        // Check if source file still exists
        if !copied_file.source_path.exists() {
            // Remove tracking record since source is already gone
            if !dry_run {
                if let Err(e) = db
                    .remove_copied_file_record(&copied_file.source_path, &copied_file.target_path)
                    .await
                {
                    eprintln!(
                        "WARNING: Failed to remove tracking record for {}: {:?}",
                        copied_file.source_path, e
                    );
                }
            }
            return Ok(SingleCleanupResult::Skipped(
                "source file already deleted".to_string(),
            ));
        }

        if dry_run {
            return Ok(SingleCleanupResult::Deleted);
        }

        // Delete the source file
        fs::remove_file(&copied_file.source_path)
            .change_context(AppError::Cleanup)
            .attach_printable_lazy(|| {
                format!("Failed to delete source file: {}", copied_file.source_path)
            })?;

        // Remove tracking record
        if let Err(e) = db
            .remove_copied_file_record(&copied_file.source_path, &copied_file.target_path)
            .await
        {
            eprintln!(
                "WARNING: Failed to remove tracking record for {}: {:?}",
                copied_file.source_path, e
            );
        }

        Ok(SingleCleanupResult::Deleted)
    }

    async fn cleanup_duplicate_file(
        &self,
        duplicate_file: &DuplicateFileRecord,
        db: &Database,
        scanner: &FileScanner,
        dry_run: bool,
    ) -> Result<SingleCleanupResult, AppError> {
        // Check if duplicate file still exists
        if !duplicate_file.duplicate_path.exists() {
            // Remove tracking record since duplicate is already gone
            if !dry_run {
                if let Err(e) = db
                    .remove_duplicate_file_record(&duplicate_file.duplicate_path)
                    .await
                {
                    eprintln!(
                        "WARNING: Failed to remove tracking record for {}: {:?}",
                        duplicate_file.duplicate_path, e
                    );
                }
            }
            return Ok(SingleCleanupResult::Skipped(
                "duplicate file already deleted".to_string(),
            ));
        }

        // Verify original file still exists and has correct hash before deleting duplicate
        if !dry_run {
            if !duplicate_file.original_path.exists() {
                return Ok(SingleCleanupResult::Skipped(
                    "original file no longer exists; refusing to delete duplicate".to_string(),
                ));
            }

            let original_hash = scanner.calculate_file_hash(&duplicate_file.original_path)
                .change_context(AppError::Cleanup)
                .attach_printable_lazy(|| {
                    format!("Failed to calculate hash for original file: {}", duplicate_file.original_path)
                })?;
            
            if original_hash != duplicate_file.hash {
                return Ok(SingleCleanupResult::Skipped(
                    "original hash mismatch; refusing to delete duplicate".to_string(),
                ));
            }
        }

        if dry_run {
            return Ok(SingleCleanupResult::Deleted);
        }

        // Delete the duplicate file
        fs::remove_file(&duplicate_file.duplicate_path)
            .change_context(AppError::Cleanup)
            .attach_printable_lazy(|| {
                format!(
                    "Failed to delete duplicate file: {}",
                    duplicate_file.duplicate_path
                )
            })?;

        // Remove tracking record
        if let Err(e) = db
            .remove_duplicate_file_record(&duplicate_file.duplicate_path)
            .await
        {
            eprintln!(
                "WARNING: Failed to remove tracking record for {}: {:?}",
                duplicate_file.duplicate_path, e
            );
        }

        Ok(SingleCleanupResult::Deleted)
    }
}
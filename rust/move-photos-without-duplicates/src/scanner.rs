use crate::db::Database;
use crate::progress::ProgressManager;
use crate::types::{AppError, FileInfo, ScanResult};
use camino::Utf8PathBuf;
use error_stack::{Result, ResultExt};
use indicatif::MultiProgress;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::sync::Arc;
use walkdir::WalkDir;

pub struct FileScanner {
    batch_size: usize,
}

impl FileScanner {
    pub fn new(batch_size: usize) -> Self {
        Self { batch_size }
    }

    pub async fn scan_and_hash_directory(
        &self,
        directory: &Utf8PathBuf,
        db: &Database,
        multi: &MultiProgress,
    ) -> Result<ScanResult, AppError> {
        let mut result = ScanResult::new();

        // Phase 1: Directory scanning with spinner
        let scan_pb = ProgressManager::create_scan_progress(multi, &format!("Scanning directory: {}", directory));

        // Collect all entries and handle walkdir errors
        let mut file_paths = Vec::new();

        for entry in WalkDir::new(directory) {
            scan_pb.tick();
            match entry {
                Ok(dir_entry) => {
                    if dir_entry.file_type().is_file() {
                        match Utf8PathBuf::from_path_buf(dir_entry.path().to_path_buf()) {
                            Ok(utf8_path) => {
                                file_paths.push(utf8_path);
                                scan_pb.set_message(format!("Scanning... found {} files", file_paths.len()));
                            }
                            Err(path_buf) => {
                                eprintln!(
                                    "ERROR: Non-UTF8 path encountered during path conversion: {:?}",
                                    path_buf
                                );
                                result.traversal_errors += 1;
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("ERROR: Directory traversal failed");
                    if let Some(path) = err.path() {
                        eprintln!("  Path: {:?}", path);
                    }
                    if let Some(io_err) = err.io_error() {
                        eprintln!(
                            "  IO Error during directory read/opendir operation: {}",
                            io_err
                        );
                    } else {
                        eprintln!("  Error: {}", err);
                    }
                    result.traversal_errors += 1;
                }
            }
        }

        if result.traversal_errors > 0 {
            eprintln!(
                "WARNING: Encountered {} errors during directory traversal",
                result.traversal_errors
            );
        }

        scan_pb.finish_with_message(format!("✓ Found {} files to process", file_paths.len()));

        // Phase 2: Batched file processing with progress bar
        let process_pb = ProgressManager::create_process_progress(multi, file_paths.len() as u64);
        process_pb.set_message("Processing files in batches...");

        let process_pb_arc = Arc::new(process_pb);
        let total_files = file_paths.len();
        let num_batches = (total_files + self.batch_size - 1) / self.batch_size;

        println!(
            "Processing {} files in {} batches of {} files each",
            total_files, num_batches, self.batch_size
        );

        // Process files in batches
        for (batch_num, batch_paths) in file_paths.chunks(self.batch_size).enumerate() {
            let batch_start = batch_num * self.batch_size;
            process_pb_arc.set_message(format!(
                "Processing batch {}/{} (files {}-{})",
                batch_num + 1,
                num_batches,
                batch_start + 1,
                batch_start + batch_paths.len()
            ));

            // Process files in this batch in parallel
            let batch_results: Vec<Result<FileInfo, AppError>> = batch_paths
                .par_iter()
                .map(|path| {
                    let result = self.process_file(path);
                    process_pb_arc.inc(1);
                    result
                })
                .collect();

            // Separate successful results from errors
            let mut batch_file_infos = Vec::new();

            for (path, file_result) in batch_paths.iter().zip(batch_results.iter()) {
                match file_result {
                    Ok(info) => {
                        batch_file_infos.push(info.clone());
                    }
                    Err(e) => {
                        eprintln!("ERROR: Failed to process file: {}", path);
                        eprintln!("  Details: {:?}", e);
                        result.processing_errors += 1;
                    }
                }
            }

            result.files_processed += batch_file_infos.len();

            // Store this batch in database using transaction
            if !batch_file_infos.is_empty() {
                match db.batch_upsert_file_hashes(&batch_file_infos).await {
                    Ok(()) => {
                        process_pb_arc.set_message(format!(
                            "✓ Batch {}/{} stored ({} files)",
                            batch_num + 1,
                            num_batches,
                            batch_file_infos.len()
                        ));
                    }
                    Err(e) => {
                        eprintln!("ERROR: Failed to store batch {} in database", batch_num + 1);
                        eprintln!("  Details: {:?}", e);
                        result.db_errors += batch_file_infos.len();
                    }
                }
            }

            // Brief pause between batches to prevent overwhelming the system
            if batch_num < num_batches - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }

        process_pb_arc.finish_with_message(format!(
            "✓ Processed {} files in {} batches",
            result.files_processed, num_batches
        ));

        // Report summary
        if result.total_errors() > 0 {
            eprintln!("SUMMARY: Completed with {} total errors", result.total_errors());
            eprintln!("  - Directory traversal errors: {}", result.traversal_errors);
            eprintln!("  - File processing errors: {}", result.processing_errors);
            eprintln!("  - Database storage errors: {}", result.db_errors);
        } else {
            println!(
                "SUMMARY: All {} files processed successfully in {} batches!",
                result.files_processed, num_batches
            );
        }

        Ok(result)
    }

    fn process_file(&self, entry_path: &Utf8PathBuf) -> Result<FileInfo, AppError> {
        let metadata = fs::metadata(entry_path)
            .change_context(AppError::IO)
            .attach_printable_lazy(|| {
                format!(
                    "Failed to get metadata (stat operation) for: {}",
                    entry_path
                )
            })?;

        let hash = self.calculate_file_hash(entry_path)?;
        let size = metadata.len();

        let last_modified = metadata
            .modified()
            .change_context(AppError::IO)
            .attach_printable_lazy(|| {
                format!(
                    "Failed to get modification time (stat.st_mtime) for: {}",
                    entry_path
                )
            })?;

        Ok(FileInfo {
            path: entry_path.clone(),
            hash,
            size,
            last_modified,
        })
    }

    pub fn calculate_file_hash(&self, file_path: &Utf8PathBuf) -> Result<String, AppError> {
        let contents = fs::read(file_path)
            .change_context(AppError::IO)
            .attach_printable_lazy(|| format!("Failed to read file contents: {}", file_path))?;

        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let hash = hasher.finalize();

        Ok(format!("{:x}", hash))
    }
}
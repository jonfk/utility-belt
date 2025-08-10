mod db;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use db::Database;
use error_stack::{Result, ResultExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error")]
    DB,

    #[error("IO error")]
    IO,

    #[error("File processing error")]
    FileProcessing,

    #[error("Directory traversal error")]
    DirectoryTraversal,

    #[error("Validation error")]
    Validation,

    #[error("Copy operation error")]
    Copy,
}

#[derive(Parser)]
#[command(name = "move-photos-without-duplicates")]
#[command(about = "Move photos from source to target directory without duplicates")]
struct Args {
    /// Custom database file location
    #[arg(long, default_value = "photo_hashes.db")]
    db_path: Utf8PathBuf,

    /// Batch size for processing files (to control memory usage)
    #[arg(long, default_value = "2000")]
    batch_size: usize,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Hash all files in target directory and store in database
    Hash {
        /// Target directory path to hash
        target_dir: Utf8PathBuf,
    },
    /// Copy files from directory A to directory B without duplicates
    #[command(alias = "cp")]
    Copy {
        /// Source directory path
        source_dir: Utf8PathBuf,
        /// Target directory path  
        target_dir: Utf8PathBuf,
        /// Directory A (within source)
        dir_a: Utf8PathBuf,
        /// Directory B (within target)
        dir_b: Utf8PathBuf,
        /// Perform a dry run without actually copying files
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Clone)]
struct FileInfo {
    path: Utf8PathBuf,
    hash: String,
    size: u64,
    last_modified: SystemTime,
}

fn calculate_file_hash(file_path: &Utf8PathBuf) -> Result<String, AppError> {
    let contents = fs::read(file_path)
        .change_context(AppError::IO)
        .attach_printable_lazy(|| format!("Failed to read file contents: {}", file_path))?;

    let mut hasher = Sha256::new();
    hasher.update(&contents);
    let hash = hasher.finalize();

    Ok(format!("{:x}", hash))
}

fn process_file(entry_path: &Utf8PathBuf) -> Result<FileInfo, AppError> {
    let metadata = fs::metadata(entry_path)
        .change_context(AppError::IO)
        .attach_printable_lazy(|| {
            format!(
                "Failed to get metadata (stat operation) for: {}",
                entry_path
            )
        })?;

    let hash = calculate_file_hash(entry_path)?;
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

fn setup_progress_logging() -> MultiProgress {
    let multi = MultiProgress::new();
    
    // For now, just return MultiProgress - we'll keep logging simple
    multi
}

async fn scan_and_hash_directory(directory: &Utf8PathBuf, db: &Database, multi: &MultiProgress, batch_size: usize) -> Result<(), AppError> {
    // Phase 1: Directory scanning with spinner
    let scan_pb = multi.add(ProgressBar::new_spinner());
    scan_pb.set_style(
        ProgressStyle::with_template("🔍 {spinner:.green} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
    );
    scan_pb.set_message(format!("Scanning directory: {}", directory));

    // Collect all entries and handle walkdir errors
    let mut file_paths = Vec::new();
    let mut traversal_errors = 0;

    for entry in WalkDir::new(directory) {
        scan_pb.tick();
        match entry {
            Ok(dir_entry) => {
                if dir_entry.file_type().is_file() {
                    match Utf8PathBuf::from_path_buf(dir_entry.path().to_path_buf()) {
                        Ok(utf8_path) => {
                            file_paths.push(utf8_path);
                            scan_pb.set_message(format!("Scanning... found {} files", file_paths.len()));
                        },
                        Err(path_buf) => {
                            eprintln!(
                                "ERROR: Non-UTF8 path encountered during path conversion: {:?}",
                                path_buf
                            );
                            traversal_errors += 1;
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
                traversal_errors += 1;
            }
        }
    }

    if traversal_errors > 0 {
        eprintln!(
            "WARNING: Encountered {} errors during directory traversal",
            traversal_errors
        );
    }

    scan_pb.finish_with_message(format!("✓ Found {} files to process", file_paths.len()));

    // Phase 2: Batched file processing with progress bar
    let process_pb = multi.add(ProgressBar::new(file_paths.len() as u64));
    process_pb.set_style(
        ProgressStyle::with_template("📁 [{elapsed}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    process_pb.set_message("Processing files in batches...");

    let process_pb_arc = Arc::new(process_pb);
    let total_files = file_paths.len();
    let num_batches = (total_files + batch_size - 1) / batch_size;
    
    let mut total_processing_errors = 0;
    let mut total_db_errors = 0;
    let mut total_processed = 0;

    println!("Processing {} files in {} batches of {} files each", total_files, num_batches, batch_size);

    // Process files in batches
    for (batch_num, batch_paths) in file_paths.chunks(batch_size).enumerate() {
        let batch_start = batch_num * batch_size;
        process_pb_arc.set_message(format!("Processing batch {}/{} (files {}-{})", 
            batch_num + 1, num_batches, batch_start + 1, batch_start + batch_paths.len()));

        // Process files in this batch in parallel
        let batch_results: Vec<Result<FileInfo, AppError>> = batch_paths
            .par_iter()
            .map(|path| {
                let result = process_file(path);
                process_pb_arc.inc(1);
                result
            })
            .collect();

        // Separate successful results from errors
        let mut batch_file_infos = Vec::new();
        let mut batch_processing_errors = 0;

        for (path, result) in batch_paths.iter().zip(batch_results.iter()) {
            match result {
                Ok(info) => {
                    batch_file_infos.push(info.clone());
                }
                Err(e) => {
                    eprintln!("ERROR: Failed to process file: {}", path);
                    eprintln!("  Details: {:?}", e);
                    batch_processing_errors += 1;
                }
            }
        }

        total_processing_errors += batch_processing_errors;
        total_processed += batch_file_infos.len();

        // Store this batch in database using transaction
        if !batch_file_infos.is_empty() {
            match db.batch_upsert_file_hashes(&batch_file_infos).await {
                Ok(()) => {
                    process_pb_arc.set_message(format!("✓ Batch {}/{} stored ({} files)", 
                        batch_num + 1, num_batches, batch_file_infos.len()));
                }
                Err(e) => {
                    eprintln!("ERROR: Failed to store batch {} in database", batch_num + 1);
                    eprintln!("  Details: {:?}", e);
                    total_db_errors += batch_file_infos.len();
                }
            }
        }

        // Brief pause between batches to prevent overwhelming the system
        if batch_num < num_batches - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    process_pb_arc.finish_with_message(format!("✓ Processed {} files in {} batches", total_processed, num_batches));

    // Report summary
    let total_errors = traversal_errors + total_processing_errors + total_db_errors;
    if total_errors > 0 {
        eprintln!("SUMMARY: Completed with {} total errors", total_errors);
        eprintln!("  - Directory traversal errors: {}", traversal_errors);
        eprintln!("  - File processing errors: {}", total_processing_errors);
        eprintln!("  - Database storage errors: {}", total_db_errors);
    } else {
        println!("SUMMARY: All {} files processed successfully in {} batches!", total_processed, num_batches);
    }

    Ok(())
}

fn validate_copy_command(
    source_dir: &Utf8PathBuf,
    target_dir: &Utf8PathBuf,
    dir_a: &Utf8PathBuf,
    dir_b: &Utf8PathBuf,
) -> Result<(), AppError> {
    // Check that all paths are directories
    if !source_dir.is_dir() {
        return Err(
            error_stack::Report::new(AppError::Validation).attach_printable(format!(
                "Source directory does not exist or is not a directory: {}",
                source_dir
            )),
        );
    }

    if !target_dir.is_dir() {
        return Err(
            error_stack::Report::new(AppError::Validation).attach_printable(format!(
                "Target directory does not exist or is not a directory: {}",
                target_dir
            )),
        );
    }

    if !dir_a.is_dir() {
        return Err(
            error_stack::Report::new(AppError::Validation).attach_printable(format!(
                "Directory A does not exist or is not a directory: {}",
                dir_a
            )),
        );
    }

    if !dir_b.is_dir() {
        return Err(
            error_stack::Report::new(AppError::Validation).attach_printable(format!(
                "Directory B does not exist or is not a directory: {}",
                dir_b
            )),
        );
    }

    // Check that dir_a is within source_dir
    let canonical_source = source_dir
        .canonicalize_utf8()
        .change_context(AppError::Validation)
        .attach_printable_lazy(|| {
            format!("Failed to canonicalize source directory: {}", source_dir)
        })?;

    let canonical_dir_a = dir_a
        .canonicalize_utf8()
        .change_context(AppError::Validation)
        .attach_printable_lazy(|| format!("Failed to canonicalize directory A: {}", dir_a))?;

    if !canonical_dir_a.starts_with(&canonical_source) {
        return Err(
            error_stack::Report::new(AppError::Validation).attach_printable(format!(
                "Directory A ({}) must be within source directory ({})",
                dir_a, source_dir
            )),
        );
    }

    // Check that dir_b is within target_dir
    let canonical_target = target_dir
        .canonicalize_utf8()
        .change_context(AppError::Validation)
        .attach_printable_lazy(|| {
            format!("Failed to canonicalize target directory: {}", target_dir)
        })?;

    let canonical_dir_b = dir_b
        .canonicalize_utf8()
        .change_context(AppError::Validation)
        .attach_printable_lazy(|| format!("Failed to canonicalize directory B: {}", dir_b))?;

    if !canonical_dir_b.starts_with(&canonical_target) {
        return Err(
            error_stack::Report::new(AppError::Validation).attach_printable(format!(
                "Directory B ({}) must be within target directory ({})",
                dir_b, target_dir
            )),
        );
    }

    println!("Validation passed:");
    println!("  Source directory: {}", source_dir);
    println!("  Target directory: {}", target_dir);
    println!("  Directory A (within source): {}", dir_a);
    println!("  Directory B (within target): {}", dir_b);

    Ok(())
}

async fn copy_files_without_duplicates(
    dir_a: &Utf8PathBuf,
    dir_b: &Utf8PathBuf,
    db: &Database,
    multi: &MultiProgress,
    dry_run: bool,
) -> Result<(), AppError> {
    println!("Starting copy operation from {} to {}", dir_a, dir_b);
    if dry_run {
        println!("DRY RUN MODE: No files will actually be copied");
    }

    // Phase 1: Directory scanning with spinner
    let scan_pb = multi.add(ProgressBar::new_spinner());
    scan_pb.set_style(
        ProgressStyle::with_template("🔍 {spinner:.green} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
    );
    scan_pb.set_message(format!("Scanning source directory: {}", dir_a));

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
                            scan_pb.set_message(format!("Scanning... found {} files", file_paths.len()));
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
    let process_pb = multi.add(ProgressBar::new(file_paths.len() as u64));
    process_pb.set_style(
        ProgressStyle::with_template("📋 [{elapsed}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    process_pb.set_message("Processing files...");

    let mut copied_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    for file_path in file_paths {
        match process_single_file(&file_path, dir_a, dir_b, db, dry_run).await {
            Ok(CopyResult::Copied) => {
                copied_count += 1;
                process_pb.set_message(format!("Copied: {} files, {} skipped, {} errors", 
                    copied_count, skipped_count, error_count));
                println!("COPIED: {}", file_path);
            }
            Ok(CopyResult::Skipped(reason)) => {
                skipped_count += 1;
                process_pb.set_message(format!("Copied: {} files, {} skipped, {} errors", 
                    copied_count, skipped_count, error_count));
                println!("SKIPPED: {} ({})", file_path, reason);
            }
            Err(e) => {
                error_count += 1;
                process_pb.set_message(format!("Copied: {} files, {} skipped, {} errors", 
                    copied_count, skipped_count, error_count));
                eprintln!("ERROR: Failed to process {}", file_path);
                eprintln!("  Details: {:?}", e);
            }
        }
        process_pb.inc(1);
    }

    process_pb.finish_with_message(format!("✓ Copy complete: {} copied, {} skipped, {} errors", 
        copied_count, skipped_count, error_count));

    // Print summary
    println!("\nCOPY OPERATION SUMMARY:");
    println!("  Files copied: {}", copied_count);
    println!("  Files skipped: {}", skipped_count);
    println!("  Errors: {}", error_count);

    if dry_run {
        println!("  (This was a dry run - no files were actually copied)");
    }

    Ok(())
}

#[derive(Debug)]
enum CopyResult {
    Copied,
    Skipped(String),
}

async fn process_single_file(
    file_path: &Utf8PathBuf,
    source_base: &Utf8PathBuf,
    target_base: &Utf8PathBuf,
    db: &Database,
    dry_run: bool,
) -> Result<CopyResult, AppError> {
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
        return Ok(CopyResult::Skipped(
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
        .attach_printable_lazy(|| format!("Failed to get modification time for: {}", file_path))?;

    let file_hash = calculate_file_hash(file_path)?;

    // Check if this hash already exists in the database (duplicate)
    let hash_exists = db
        .hash_exists(&file_hash)
        .await
        .change_context(AppError::DB)
        .attach_printable_lazy(|| format!("file={file_path}"))?;

    if hash_exists {
        return Ok(CopyResult::Skipped(
            "duplicate content (hash exists)".to_string(),
        ));
    }

    if dry_run {
        return Ok(CopyResult::Copied);
    }

    // Create target directory if it doesn't exist
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .change_context(AppError::Copy)
            .attach_printable_lazy(|| format!("Failed to create target directory: {}", parent))?;
    }

    // Copy the file
    fs::copy(file_path, &target_path)
        .change_context(AppError::Copy)
        .attach_printable_lazy(|| format!("Failed to copy {} to {}", file_path, target_path))?;

    // Store hash in database for the new file
    let file_size = metadata.len();
    if let Err(e) = db
        .upsert_file_hash(&target_path, &file_hash, file_size, last_modified)
        .await
    {
        eprintln!(
            "WARNING: Failed to store hash for copied file {}: {:?}",
            target_path, e
        );
    }

    Ok(CopyResult::Copied)
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = Args::parse();

    // Setup progress bars and logging
    let multi = setup_progress_logging();

    // Initialize database
    let db = Database::new(&args.db_path)
        .await
        .change_context(AppError::DB)?;

    println!("Database initialized successfully");

    match args.command {
        Commands::Hash { target_dir } => {
            println!("Running hash command for target directory: {}", target_dir);
            scan_and_hash_directory(&target_dir, &db, &multi, args.batch_size).await?;
            println!("Directory scanning and hashing completed");
        }
        Commands::Copy {
            source_dir,
            target_dir,
            dir_a,
            dir_b,
            dry_run,
        } => {
            println!("Running copy command");
            validate_copy_command(&source_dir, &target_dir, &dir_a, &dir_b)?;
            copy_files_without_duplicates(&dir_a, &dir_b, &db, &multi, dry_run).await?;
            println!("Copy operation completed");
        }
    }

    Ok(())
}

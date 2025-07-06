mod db;

use camino::Utf8PathBuf;
use clap::Parser;
use db::Database;
use error_stack::{Result, ResultExt};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::fs;
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
}

#[derive(Parser)]
#[command(name = "move-photos-without-duplicates")]
#[command(about = "Move photos from source to target directory without duplicates")]
struct Args {
    /// Source directory path
    source_dir: Utf8PathBuf,

    /// Target directory path  
    target_dir: Utf8PathBuf,

    /// Directory A (within source or source itself)
    dir_a: Utf8PathBuf,

    /// Directory B (within target or target itself)
    dir_b: Utf8PathBuf,

    /// Custom database file location
    #[arg(long, default_value = "photo_hashes.db")]
    db_path: Utf8PathBuf,
}

#[derive(Debug)]
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
        .attach_printable_lazy(|| format!("Failed to get metadata (stat operation) for: {}", entry_path))?;
    
    let hash = calculate_file_hash(entry_path)?;
    let size = metadata.len();
    
    let last_modified = metadata.modified()
        .change_context(AppError::IO)
        .attach_printable_lazy(|| format!("Failed to get modification time (stat.st_mtime) for: {}", entry_path))?;
    
    Ok(FileInfo {
        path: entry_path.clone(),
        hash,
        size,
        last_modified,
    })
}

async fn scan_and_hash_directory(
    directory: &Utf8PathBuf,
    db: &Database,
) -> Result<(), AppError> {
    println!("Scanning directory: {}", directory);
    
    // Collect all entries and handle walkdir errors
    let mut file_paths = Vec::new();
    let mut traversal_errors = 0;
    
    for entry in WalkDir::new(directory) {
        match entry {
            Ok(dir_entry) => {
                if dir_entry.file_type().is_file() {
                    match Utf8PathBuf::from_path_buf(dir_entry.path().to_path_buf()) {
                        Ok(utf8_path) => file_paths.push(utf8_path),
                        Err(path_buf) => {
                            eprintln!("ERROR: Non-UTF8 path encountered during path conversion: {:?}", path_buf);
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
                    eprintln!("  IO Error during directory read/opendir operation: {}", io_err);
                } else {
                    eprintln!("  Error: {}", err);
                }
                traversal_errors += 1;
            }
        }
    }
    
    if traversal_errors > 0 {
        eprintln!("WARNING: Encountered {} errors during directory traversal", traversal_errors);
    }
    
    println!("Found {} files to process", file_paths.len());
    
    // Process files in parallel using rayon
    let results: Vec<Result<FileInfo, AppError>> = file_paths
        .par_iter()
        .map(|path| process_file(path))
        .collect();
    
    // Separate successful results from errors
    let mut file_infos = Vec::new();
    let mut processing_errors = 0;
    
    for (path, result) in file_paths.iter().zip(results.iter()) {
        match result {
            Ok(info) => {
                println!("Processed: {}", path);
                file_infos.push(info);
            }
            Err(e) => {
                eprintln!("ERROR: Failed to process file: {}", path);
                eprintln!("  Details: {:?}", e);
                processing_errors += 1;
            }
        }
    }
    
    if processing_errors > 0 {
        eprintln!("WARNING: Failed to process {} files", processing_errors);
    }
    
    println!("Successfully processed {} files", file_infos.len());
    
    // Store results in database sequentially (SQLite doesn't handle concurrent writes well)
    let mut db_errors = 0;
    for file_info in file_infos {
        match db.upsert_file_hash(
            &file_info.path,
            &file_info.hash,
            file_info.size,
            file_info.last_modified,
        ).await {
            Ok(()) => {},
            Err(e) => {
                eprintln!("ERROR: Failed to store hash in database (SQL INSERT/UPDATE operation)");
                eprintln!("  File: {}", file_info.path);
                eprintln!("  Database error: {:?}", e);
                db_errors += 1;
            }
        }
    }
    
    if db_errors > 0 {
        eprintln!("WARNING: Failed to store {} file hashes in database", db_errors);
    }
    
    // Report summary
    let total_errors = traversal_errors + processing_errors + db_errors;
    if total_errors > 0 {
        eprintln!("SUMMARY: Completed with {} total errors", total_errors);
        eprintln!("  - Directory traversal errors: {}", traversal_errors);
        eprintln!("  - File processing errors: {}", processing_errors);
        eprintln!("  - Database storage errors: {}", db_errors);
    } else {
        println!("SUMMARY: All files processed successfully!");
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = Args::parse();

    // Initialize database
    let db = Database::new(&args.db_path)
        .await
        .change_context(AppError::DB)?;

    println!("Database initialized successfully");

    // Scan and hash the target directory
    scan_and_hash_directory(&args.target_dir, &db).await?;
    
    println!("Directory scanning and hashing completed");

    Ok(())
}

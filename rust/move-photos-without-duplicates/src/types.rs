use camino::Utf8PathBuf;
use std::time::SystemTime;
use thiserror::Error;

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

    #[error("Cleanup operation error")]
    Cleanup,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: Utf8PathBuf,
    pub hash: String,
    pub size: u64,
    pub last_modified: SystemTime,
}

#[derive(Debug)]
pub enum SingleCopyResult {
    Copied,
    Skipped(String),
}

#[derive(Debug)]
pub enum SingleCleanupResult {
    Deleted,
    Skipped(String),
}

pub struct ScanResult {
    pub files_processed: usize,
    pub traversal_errors: usize,
    pub processing_errors: usize,
    pub db_errors: usize,
}

impl ScanResult {
    pub fn new() -> Self {
        Self {
            files_processed: 0,
            traversal_errors: 0,
            processing_errors: 0,
            db_errors: 0,
        }
    }

    pub fn total_errors(&self) -> usize {
        self.traversal_errors + self.processing_errors + self.db_errors
    }
}

pub struct CopyOperationResult {
    pub copied: usize,
    pub skipped: usize,
    pub errors: usize,
}

impl CopyOperationResult {
    pub fn new() -> Self {
        Self {
            copied: 0,
            skipped: 0,
            errors: 0,
        }
    }
}

pub struct CleanupOperationResult {
    pub deleted: usize,
    pub skipped: usize,
    pub errors: usize,
}

impl CleanupOperationResult {
    pub fn new() -> Self {
        Self {
            deleted: 0,
            skipped: 0,
            errors: 0,
        }
    }
}
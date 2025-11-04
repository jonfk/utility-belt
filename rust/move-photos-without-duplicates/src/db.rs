use crate::types::FileInfo;
use camino::Utf8PathBuf;
use error_stack::{Result, ResultExt};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use std::time::SystemTime;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Failed to connect to database")]
    Connection,

    #[error("Failed to run migration")]
    Migration,

    #[error("Failed to insert file hash")]
    Insert,

    #[error("Failed to query hash")]
    QueryHashExists,

    #[error("Failed to query file hash")]
    QueryGetFileHash,

    #[error("Failed to delete file hash")]
    Delete,

    #[error("Failed to query files by filename")]
    QueryGetFilesByFilename,

    #[error("Failed to query files by hash")]
    QueryGetFilesByHash,

    #[error("Failed to track copied file")]
    TrackCopiedFile,

    #[error("Failed to track duplicate file")]
    TrackDuplicateFile,

    #[error("Failed to query copied files")]
    QueryCopiedFiles,

    #[error("Failed to query duplicate files")]
    QueryDuplicateFiles,

    #[error("Failed to remove tracked file")]
    RemoveTrackedFile,
}

const MIGRATION_SQL: &str = r#"
-- Create file_hashes table to store file content hashes
CREATE TABLE IF NOT EXISTS file_hashes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL UNIQUE,
    filename TEXT NOT NULL,
    hash TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    last_modified INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Index for efficient hash lookups (checking for duplicates)
CREATE INDEX IF NOT EXISTS idx_hash ON file_hashes(hash);

-- Index for efficient filename lookups
CREATE INDEX IF NOT EXISTS idx_filename ON file_hashes(filename);

-- Index for efficient filepath lookups (though filepath is unique)
CREATE INDEX IF NOT EXISTS idx_filepath ON file_hashes(file_path);

-- Index for efficient cleanup of stale entries
CREATE INDEX IF NOT EXISTS idx_last_modified ON file_hashes(last_modified);

-- Create copied_files table to track successfully copied files
CREATE TABLE IF NOT EXISTS copied_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_path TEXT NOT NULL,
    target_path TEXT NOT NULL,
    hash TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Create duplicate_files table to track files skipped as duplicates
CREATE TABLE IF NOT EXISTS duplicate_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    duplicate_path TEXT NOT NULL,
    original_path TEXT NOT NULL,
    hash TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_copied_source ON copied_files(source_path);
CREATE INDEX IF NOT EXISTS idx_copied_target ON copied_files(target_path);
CREATE INDEX IF NOT EXISTS idx_copied_hash ON copied_files(hash);
CREATE INDEX IF NOT EXISTS idx_duplicate_path ON duplicate_files(duplicate_path);
CREATE INDEX IF NOT EXISTS idx_duplicate_original ON duplicate_files(original_path);
CREATE INDEX IF NOT EXISTS idx_duplicate_hash ON duplicate_files(hash);
"#;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(db_path: &Utf8PathBuf) -> Result<Self, DatabaseError> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .change_context(DatabaseError::Connection)
                .attach_printable_lazy(|| {
                    format!("Failed to create database directory: {}", parent)
                })?;
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))
            .change_context(DatabaseError::Connection)
            .attach_printable_lazy(|| format!("Failed to parse database URL for: {}", db_path))?
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .change_context(DatabaseError::Connection)
            .attach_printable_lazy(|| format!("db_path={}", db_path))?;

        // Run migration
        sqlx::query(MIGRATION_SQL)
            .execute(&pool)
            .await
            .change_context(DatabaseError::Migration)?;

        Ok(Database { pool })
    }

    /// Insert or update a file hash entry
    pub(crate) async fn upsert_file_hash(
        &self,
        file_path: &Utf8PathBuf,
        hash: &str,
        file_size: u64,
        last_modified: SystemTime,
    ) -> Result<(), DatabaseError> {
        let filename = file_path.file_name().expect(&format!(
            "Unexpectedly could not get filename. file_path = {}",
            file_path
        ));

        let last_modified_secs = last_modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Could not get Unix seconds timestamp from SystemTime")
            .as_secs() as i64;

        sqlx::query(
            r#"
            INSERT INTO file_hashes (file_path, filename, hash, file_size, last_modified)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(file_path) DO UPDATE SET
                filename = excluded.filename,
                hash = excluded.hash,
                file_size = excluded.file_size,
                last_modified = excluded.last_modified
            "#,
        )
        .bind(file_path.as_str())
        .bind(filename)
        .bind(hash)
        .bind(file_size as i64)
        .bind(last_modified_secs)
        .execute(&self.pool)
        .await
        .change_context(DatabaseError::Insert)?;

        Ok(())
    }

    /// Insert or update multiple file hash entries in a single transaction
    pub async fn batch_upsert_file_hashes(
        &self,
        file_infos: &[FileInfo],
    ) -> Result<(), DatabaseError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .change_context(DatabaseError::Insert)?;

        for file_info in file_infos {
            let filename = file_info.path.file_name().unwrap_or("unknown");
            let last_modified_secs = file_info
                .last_modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            sqlx::query(
                r#"
                INSERT INTO file_hashes (file_path, filename, hash, file_size, last_modified)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(file_path) DO UPDATE SET
                    filename = excluded.filename,
                    hash = excluded.hash,
                    file_size = excluded.file_size,
                    last_modified = excluded.last_modified
                "#,
            )
            .bind(file_info.path.as_str())
            .bind(filename)
            .bind(&file_info.hash)
            .bind(file_info.size as i64)
            .bind(last_modified_secs)
            .execute(&mut *tx)
            .await
            .change_context(DatabaseError::Insert)?;
        }

        tx.commit().await.change_context(DatabaseError::Insert)?;
        Ok(())
    }

    /// Check if a hash already exists in the database
    pub async fn hash_exists(&self, hash: &str) -> Result<bool, DatabaseError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM file_hashes WHERE hash = ?")
            .bind(hash)
            .fetch_one(&self.pool)
            .await
            .change_context(DatabaseError::QueryHashExists)?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }

    /// Get all file paths that have the same hash
    pub async fn get_files_with_hash(&self, hash: &str) -> Result<Vec<Utf8PathBuf>, DatabaseError> {
        let rows = sqlx::query("SELECT file_path FROM file_hashes WHERE hash = ?")
            .bind(hash)
            .fetch_all(&self.pool)
            .await
            .change_context(DatabaseError::QueryGetFilesByHash)?;

        let paths = rows
            .into_iter()
            .map(|row| {
                let path_str: String = row.get("file_path");
                Utf8PathBuf::from(path_str)
            })
            .collect();

        Ok(paths)
    }

    /// Get all files with the same filename (regardless of path)
    pub async fn get_files_with_filename(
        &self,
        filename: &str,
    ) -> Result<Vec<Utf8PathBuf>, DatabaseError> {
        let rows = sqlx::query("SELECT file_path FROM file_hashes WHERE filename = ?")
            .bind(filename)
            .fetch_all(&self.pool)
            .await
            .change_context(DatabaseError::QueryGetFilesByFilename)?;

        let paths = rows
            .into_iter()
            .map(|row| {
                let path_str: String = row.get("file_path");
                Utf8PathBuf::from(path_str)
            })
            .collect();

        Ok(paths)
    }

    /// Remove entries for files that no longer exist or have been modified
    pub async fn remove_stale_entry(&self, file_path: &Utf8PathBuf) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM file_hashes WHERE file_path = ?")
            .bind(file_path.as_str())
            .execute(&self.pool)
            .await
            .change_context(DatabaseError::Delete)?;

        Ok(())
    }

    /// Get file hash entry if it exists and is current
    pub async fn get_file_hash(
        &self,
        file_path: &Utf8PathBuf,
        current_last_modified: SystemTime,
    ) -> Result<Option<String>, DatabaseError> {
        let current_modified_secs = current_last_modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let row = sqlx::query("SELECT hash, last_modified FROM file_hashes WHERE file_path = ?")
            .bind(file_path.as_str())
            .fetch_optional(&self.pool)
            .await
            .change_context(DatabaseError::QueryGetFileHash)?;

        if let Some(row) = row {
            let stored_modified: i64 = row.get("last_modified");

            // Only return hash if the file hasn't been modified
            if stored_modified == current_modified_secs {
                let hash: String = row.get("hash");
                Ok(Some(hash))
            } else {
                // File has been modified, remove stale entry
                self.remove_stale_entry(file_path).await?;
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Record a successfully copied file - atomically updates file hash and tracks copy
    pub async fn record_copied_file(
        &self,
        source_path: &Utf8PathBuf,
        target_path: &Utf8PathBuf,
        hash: &str,
        file_size: u64,
        last_modified: SystemTime,
    ) -> Result<(), DatabaseError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .change_context(DatabaseError::TrackCopiedFile)?;

        // First, upsert the file hash for the target file
        let filename = target_path.file_name().expect(&format!(
            "Unexpectedly could not get filename. file_path = {}",
            target_path
        ));

        let last_modified_secs = last_modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Could not get Unix seconds timestamp from SystemTime")
            .as_secs() as i64;

        sqlx::query(
            r#"
            INSERT INTO file_hashes (file_path, filename, hash, file_size, last_modified)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(file_path) DO UPDATE SET
                filename = excluded.filename,
                hash = excluded.hash,
                file_size = excluded.file_size,
                last_modified = excluded.last_modified
            "#,
        )
        .bind(target_path.as_str())
        .bind(filename)
        .bind(hash)
        .bind(file_size as i64)
        .bind(last_modified_secs)
        .execute(&mut *tx)
        .await
        .change_context(DatabaseError::TrackCopiedFile)?;

        // Then, track the copy operation
        sqlx::query(
            r#"
            INSERT INTO copied_files (source_path, target_path, hash, file_size)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(source_path.as_str())
        .bind(target_path.as_str())
        .bind(hash)
        .bind(file_size as i64)
        .execute(&mut *tx)
        .await
        .change_context(DatabaseError::TrackCopiedFile)?;

        tx.commit()
            .await
            .change_context(DatabaseError::TrackCopiedFile)?;
        Ok(())
    }

    /// Track a successfully copied file
    pub(crate) async fn track_copied_file(
        &self,
        source_path: &Utf8PathBuf,
        target_path: &Utf8PathBuf,
        hash: &str,
        file_size: u64,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO copied_files (source_path, target_path, hash, file_size)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(source_path.as_str())
        .bind(target_path.as_str())
        .bind(hash)
        .bind(file_size as i64)
        .execute(&self.pool)
        .await
        .change_context(DatabaseError::TrackCopiedFile)?;

        Ok(())
    }

    /// Track a file that was skipped as duplicate
    pub async fn track_duplicate_file(
        &self,
        duplicate_path: &Utf8PathBuf,
        original_path: &Utf8PathBuf,
        hash: &str,
        file_size: u64,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO duplicate_files (duplicate_path, original_path, hash, file_size)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(duplicate_path.as_str())
        .bind(original_path.as_str())
        .bind(hash)
        .bind(file_size as i64)
        .execute(&self.pool)
        .await
        .change_context(DatabaseError::TrackDuplicateFile)?;

        Ok(())
    }

    /// Get all tracked copied files
    pub async fn get_copied_files(&self) -> Result<Vec<CopiedFileRecord>, DatabaseError> {
        let rows = sqlx::query(
            "SELECT source_path, target_path, hash, file_size, created_at FROM copied_files ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await
        .change_context(DatabaseError::QueryCopiedFiles)?;

        let records = rows
            .into_iter()
            .map(|row| {
                let source_path: String = row.get("source_path");
                let target_path: String = row.get("target_path");
                let hash: String = row.get("hash");
                let file_size: i64 = row.get("file_size");
                let created_at: i64 = row.get("created_at");

                CopiedFileRecord {
                    source_path: Utf8PathBuf::from(source_path),
                    target_path: Utf8PathBuf::from(target_path),
                    hash,
                    file_size: file_size as u64,
                    created_at,
                }
            })
            .collect();

        Ok(records)
    }

    /// Get all tracked duplicate files
    pub async fn get_duplicate_files(&self) -> Result<Vec<DuplicateFileRecord>, DatabaseError> {
        let rows = sqlx::query(
            "SELECT duplicate_path, original_path, hash, file_size, created_at FROM duplicate_files ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await
        .change_context(DatabaseError::QueryDuplicateFiles)?;

        let records = rows
            .into_iter()
            .map(|row| {
                let duplicate_path: String = row.get("duplicate_path");
                let original_path: String = row.get("original_path");
                let hash: String = row.get("hash");
                let file_size: i64 = row.get("file_size");
                let created_at: i64 = row.get("created_at");

                DuplicateFileRecord {
                    duplicate_path: Utf8PathBuf::from(duplicate_path),
                    original_path: Utf8PathBuf::from(original_path),
                    hash,
                    file_size: file_size as u64,
                    created_at,
                }
            })
            .collect();

        Ok(records)
    }

    /// Remove a tracked copied file record
    pub async fn remove_copied_file_record(
        &self,
        source_path: &Utf8PathBuf,
        target_path: &Utf8PathBuf,
    ) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM copied_files WHERE source_path = ? AND target_path = ?")
            .bind(source_path.as_str())
            .bind(target_path.as_str())
            .execute(&self.pool)
            .await
            .change_context(DatabaseError::RemoveTrackedFile)?;

        Ok(())
    }

    /// Remove a tracked duplicate file record
    pub async fn remove_duplicate_file_record(
        &self,
        duplicate_path: &Utf8PathBuf,
    ) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM duplicate_files WHERE duplicate_path = ?")
            .bind(duplicate_path.as_str())
            .execute(&self.pool)
            .await
            .change_context(DatabaseError::RemoveTrackedFile)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CopiedFileRecord {
    pub source_path: Utf8PathBuf,
    pub target_path: Utf8PathBuf,
    pub hash: String,
    pub file_size: u64,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct DuplicateFileRecord {
    pub duplicate_path: Utf8PathBuf,
    pub original_path: Utf8PathBuf,
    pub hash: String,
    pub file_size: u64,
    pub created_at: i64,
}

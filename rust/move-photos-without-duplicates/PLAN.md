# Photo Deduplication CLI Tool

## Overview

A Rust CLI program that moves photo files from a source directory to a target directory while preventing duplicates based on file content (not filename).

## Requirements

### Core Functionality

- Move files from directory A to directory B
- Detect duplicates using file content hashing (same content, different names)
- Prevent copying files that already exist in the target directory
- Handle large numbers of files efficiently
- Support multiple runs with different A and B directories

### Technical Requirements

- **Language**: Rust
- **CLI Parsing**: clap
- **Database**: SQLite with sqlx
- **Hashing**: SHA-256 (robust and simple)
- **Runtime**: tokio (async, required by sqlx)

### Directory Structure

- **Source Directory**: Base source directory containing photos
- **Target Directory**: Base target directory for photos
- **Directory A**: Path within source (or source itself) - files to move from
- **Directory B**: Path within target (or target itself) - files to move to

### Persistence Strategy

- Use SQLite database to store file hashes across runs
- Cache hashes of all files in target directory
- Avoid recomputing hashes unnecessarily
- Handle target directory changes between runs

## CLI Interface

### Arguments

- `source_dir`: Source directory path
- `target_dir`: Target directory path
- `dir_a`: Directory A (within source or source itself)
- `dir_b`: Directory B (within target or target itself)

### Options

- `--db-path`: Custom database file location (optional)

## Workflow

1. **Initialize**
   - Parse CLI arguments
   - Initialize/open SQLite database
   - Create tables if they don't exist
2. **Cache Target Directory**
   - Scan target directory recursively
   - Check if files in cache are still valid (compare last_modified)
   - Compute and store hashes for new/changed files
   - Remove stale entries from cache
3. **Process Source Files**
   - Scan directory A for files
   - For each file:
     - Compute SHA-256 hash
     - Check if hash exists in target directory cache
     - If duplicate found, skip file
     - If not duplicate, move file to directory B
     - Update cache with new file location
4. **Error Handling**
   - Handle file I/O errors gracefully
   - Handle database connection issues
   - Provide meaningful error messages

## Dependencies

- clap for cli argument parsing
- sqlx for persistence to sqlite 
- sha2 for hashing with sha-256
- tokio async runtime
- walkdir for directory traversal and iterating over files
- error-stack for high level error reporting and returning errors externally to users
- thiserror to create specific errors from low level errors
- camino for utf8 paths. We can assume all the paths are utf8 since they are generally from devices that support utf8 such as iPhone, cameras, etc. If we encounter a non-utf8 path, we return and error and exit.
    - If this becomes an issue in the future, we can instead decide to either support byte strings or skip non-utf8 string and print an error when it happens. 

## Design Principles

- Keep implementation simple and minimal
- Prioritize correctness over performance optimizations
- Use async/await for database operations
- Handle edge cases gracefully
- Provide clear error messages and logging

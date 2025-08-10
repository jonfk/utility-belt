mod cleaner;
mod copier;
mod db;
mod progress;
mod scanner;
mod types;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use cleaner::FileCleaner;
use copier::FileCopier;
use db::Database;
use error_stack::{Result, ResultExt};
use progress::ProgressManager;
use scanner::FileScanner;
use types::AppError;

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
    /// Copy files from source to target directory without duplicates
    #[command(alias = "cp")]
    Copy {
        /// Source directory path
        source: Utf8PathBuf,
        /// Target directory path  
        target: Utf8PathBuf,
        /// Perform a dry run without actually copying files
        #[arg(long)]
        dry_run: bool,
    },
    /// Clean up tracked copied and duplicate files
    Cleanup {
        /// Perform a dry run without actually deleting files
        #[arg(long)]
        dry_run: bool,
    },
    /// Show status of files that would be deleted by cleanup
    Status {
        /// Show detailed list of all file paths that would be deleted
        #[arg(long)]
        detailed: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = Args::parse();

    // Setup progress bars and logging
    let multi = ProgressManager::setup();

    // Initialize database
    let db = Database::new(&args.db_path)
        .await
        .change_context(AppError::DB)?;

    println!("Database initialized successfully");

    match args.command {
        Commands::Hash { target_dir } => {
            println!("Running hash command for target directory: {}", target_dir);
            let scanner = FileScanner::new(args.batch_size);
            scanner.scan_and_hash_directory(&target_dir, &db, &multi).await?;
            println!("Directory scanning and hashing completed");
        }
        Commands::Copy {
            source,
            target,
            dry_run,
        } => {
            println!("Running copy command");
            FileCopier::validate_copy_command(&source, &target)?;
            let copier = FileCopier::new();
            copier.copy_files_without_duplicates(&source, &target, &db, &multi, dry_run).await?;
            println!("Copy operation completed");
        }
        Commands::Cleanup { dry_run } => {
            println!("Running cleanup command");
            let scanner = FileScanner::new(args.batch_size);
            let cleaner = FileCleaner::new();
            cleaner.cleanup_tracked_files(&db, &scanner, &multi, dry_run).await?;
            println!("Cleanup operation completed");
        }
        Commands::Status { detailed } => {
            let cleaner = FileCleaner::new();
            cleaner.show_status(&db, &multi, detailed).await?;
        }
    }

    Ok(())
}
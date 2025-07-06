mod db;

use camino::Utf8PathBuf;
use clap::Parser;
use db::Database;
use error_stack::Result;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize database
    let _db = Database::new(&args.db_path).await?;
    
    println!("Database initialized successfully");
    
    Ok(())
}

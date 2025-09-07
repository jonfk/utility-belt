#!/usr/bin/env uv run
# /// script
# dependencies = ["pillow>=11.3.0"]
# ///

import argparse
import os
import shutil
from datetime import datetime
from pathlib import Path
from PIL import Image
from PIL.ExifTags import TAGS


def get_date_from_exif(file_path):
    """Extract date from image EXIF data."""
    try:
        with Image.open(file_path) as img:
            exif = img.getexif()
            if exif:
                # Try different date fields in order of preference
                date_fields = ['DateTimeOriginal', 'DateTime', 'DateTimeDigitized']
                for field in date_fields:
                    for tag_id, value in exif.items():
                        tag = TAGS.get(tag_id, tag_id)
                        if tag == field:
                            try:
                                # Parse EXIF date format: "YYYY:MM:DD HH:MM:SS"
                                return datetime.strptime(value, "%Y:%m:%d %H:%M:%S")
                            except ValueError:
                                continue
    except Exception:
        pass
    return None


def get_file_date(file_path):
    """Get file creation/modification date as fallback."""
    try:
        # Use creation time if available (Windows), otherwise modification time
        stat = file_path.stat()
        if hasattr(stat, 'st_birthtime'):  # macOS
            return datetime.fromtimestamp(stat.st_birthtime)
        else:  # Linux/Windows
            return datetime.fromtimestamp(min(stat.st_ctime, stat.st_mtime))
    except Exception:
        return datetime.now()


def get_file_date_for_organization(file_path):
    """Get the best available date for organizing the file."""
    # First try EXIF for image files
    if file_path.suffix.lower() in {'.jpg', '.jpeg', '.tiff', '.tif'}:
        exif_date = get_date_from_exif(file_path)
        if exif_date:
            return exif_date
    
    # Fallback to file system date
    return get_file_date(file_path)


def organize_files(dry_run=False, verbose=False):
    """Organize files in current directory by date."""
    current_dir = Path('.')
    
    # Get all files (not directories)
    files = [f for f in current_dir.iterdir() if f.is_file() and f.name != __file__.split('/')[-1]]
    
    if not files:
        if verbose:
            print("No files found in current directory.")
        return
    
    for file_path in files:
        try:
            # Get the date for organization
            file_date = get_file_date_for_organization(file_path)
            date_dir = file_date.strftime("%Y-%m-%d")
            target_dir = Path(date_dir)
            target_path = target_dir / file_path.name
            
            if verbose:
                source_info = "EXIF" if file_path.suffix.lower() in {'.jpg', '.jpeg', '.tiff', '.tif'} and get_date_from_exif(file_path) else "file system"
                print(f"{file_path.name} -> {target_path} (date from {source_info}: {file_date.strftime('%Y-%m-%d %H:%M:%S')})")
            
            if not dry_run:
                # Create directory if it doesn't exist
                target_dir.mkdir(exist_ok=True)
                
                # Handle filename conflicts
                counter = 1
                original_target = target_path
                while target_path.exists():
                    stem = original_target.stem
                    suffix = original_target.suffix
                    target_path = target_dir / f"{stem}_{counter:03d}{suffix}"
                    counter += 1
                
                # Move the file
                shutil.move(str(file_path), str(target_path))
                
                if verbose and target_path != original_target:
                    print(f"  Renamed to avoid conflict: {target_path.name}")
        
        except Exception as e:
            if verbose:
                print(f"Error processing {file_path.name}: {e}")


def main():
    parser = argparse.ArgumentParser(
        description="""
Organize files into date-based directories using EXIF data or file dates.

EXAMPLES:
  ./organisze-files-into-dirs-by-date.py --dry-run    # Preview what would happen
  ./organisze-files-into-dirs-by-date.py -v           # Organize with verbose output

HOW IT WORKS:
  • Images (JPEG/TIFF): Uses EXIF DateTimeOriginal → DateTime → DateTimeDigitized
  • Other files: Uses file creation/modification date
  • Creates directories like: 2025-01-15/, 2024-12-03/, etc.
  • Handles filename conflicts with numbered suffixes: IMG_001.jpg, IMG_002.jpg

EXAMPLE OUTPUT:
  Before:  IMG_1234.jpg, document.pdf, video.mp4
  After:   2024-03-15/IMG_1234.jpg
           2024-03-16/document.pdf  
           2024-03-16/video.mp4
        """,
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--dry-run", 
        action="store_true", 
        help="Show what would be done without actually moving files"
    )
    parser.add_argument(
        "-v", "--verbose", 
        action="store_true", 
        help="Show detailed output"
    )
    
    args = parser.parse_args()
    
    if args.dry_run and not args.verbose:
        # Enable verbose for dry run to show what would happen
        args.verbose = True
        print("DRY RUN MODE - No files will be moved")
        print()
    
    organize_files(dry_run=args.dry_run, verbose=args.verbose)


if __name__ == "__main__":
    main()

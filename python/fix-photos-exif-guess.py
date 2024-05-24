#!/usr/bin/env python3

import os
import subprocess
import argparse
import re
import json
import fnmatch
import hashlib
import time
from collections import defaultdict
from datetime import datetime

# List of glob patterns to skip
SKIP_PATTERNS = ['*.AAE', '*.DS_Store']

def get_cache_file_name(directory, default_date):
    dir_hash = hashlib.md5(directory.encode()).hexdigest()
    if default_date:
        date_hash = hashlib.md5(default_date.encode()).hexdigest()
    else:
        date_hash = "no_default_date"
    return f"metadata_cache_{dir_hash}_{date_hash}.json"

def load_cache(cache_file):
    if os.path.exists(cache_file):
        with open(cache_file, 'r') as file:
            return json.load(file)
    return {}

def save_cache(cache, cache_file):
    with open(cache_file, 'w') as file:
        json.dump(cache, file, indent=4)

def get_metadata(file_path, debug):
    if debug:
        print(f"Extracting metadata for file: {file_path}")
    # Extract metadata using exiftool
    result = subprocess.run(
        ['exiftool', '-datetimeoriginal', '-createdate', '-modifydate', '-filemodifydate', '-filecreatedate', '-fileaccessdate', file_path],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    metadata = {}
    for line in result.stdout.split('\n'):
        if ': ' in line:
            key, value = line.split(': ', 1)
            metadata[key.strip()] = value.strip()
    return metadata

def save_metadata(directory, cache, debug):
    if debug:
        print(f"Saving metadata for files in directory: {directory}")
    metadata_info = {}
    for root, _, files in os.walk(directory):
        for filename in files:
            if any(fnmatch.fnmatch(filename, pattern) for pattern in SKIP_PATTERNS):
                if debug:
                    print(f"Skipping file {filename} based on skip patterns.")
                continue

            file_path = os.path.join(root, filename)
            if os.path.isfile(file_path) and filename not in cache:
                metadata_info[file_path] = get_metadata(file_path, debug)
                if debug:
                    print(f"Metadata for {file_path}: {metadata_info[file_path]}")
    cache.update(metadata_info)
    return cache

def filename_prefix(filename):
    match = re.match(r'([a-zA-Z]+)_\d+', filename)
    if match:
        return match.group(1)
    return None

def get_default_date(with_date):
    if with_date:
        first_date = sorted(with_date.values())[0]
        year = first_date.split(':')[0]
        return f"{year}:01:01 00:00:00"
    return None

def guess_dates(metadata_info, default_date, debug):
    if debug:
        print("Guessing dates for files without DateTimeOriginal...")
    # Separate files with and without DateTimeOriginal
    with_date = {}
    without_date = {}
    
    for file_path, metadata in metadata_info.items():
        if 'Date/Time Original' in metadata:
            with_date[file_path] = metadata['Date/Time Original']
        else:
            without_date[file_path] = metadata
    
    if not default_date:
        default_date = get_default_date(with_date)

    if debug:
        print(f"Files with DateTimeOriginal: {list(with_date.keys())}")
        print(f"Files without DateTimeOriginal: {list(without_date.keys())}")

    # Guess dates based on filenames
    guesses = {}
    sorted_files = sorted(with_date.keys())
    
    for file_path in sorted(without_date.keys()):
        filename = os.path.basename(file_path)
        closest_before = None
        closest_after = None
        filename_prefix_current = filename_prefix(filename)
        create_date = without_date[file_path].get('Create Date') or without_date[file_path].get('File Create Date')

        if create_date:
            guesses[file_path] = {'date': create_date, 'reason': "Based on file create date"}
            continue

        for ref_file in sorted_files:
            ref_file_prefix = filename_prefix(os.path.basename(ref_file))
            if filename_prefix_current and ref_file_prefix == filename_prefix_current:
                if ref_file < file_path:
                    closest_before = ref_file
                elif ref_file > file_path and closest_after is None:
                    closest_after = ref_file

        if closest_before and closest_after:
            guess_date = with_date[closest_before]  # Choose one date for simplicity
            guesses[file_path] = {'date': guess_date, 'reason': (f"Between {with_date[closest_before]} (based on {os.path.basename(closest_before)}) and {with_date[closest_after]} "
                                                                 f"(based on {os.path.basename(closest_after)}) based on filename sequence.")}
        elif closest_before:
            guess_date = with_date[closest_before]
            guesses[file_path] = {'date': guess_date, 'reason': f"After {with_date[closest_before]} (based on {os.path.basename(closest_before)}) based on filename sequence."}
        elif closest_after:
            guess_date = with_date[closest_after]
            guesses[file_path] = {'date': guess_date, 'reason': f"Before {with_date[closest_after]} (based on {os.path.basename(closest_after)}) based on filename sequence."}
        else:
            guesses[file_path] = {'date': default_date, 'reason': "Beginning of the year guessed."}
    
    if debug:
        print(f"Guessed dates: {guesses}")

    return guesses

def main(directory, default_date, debug, execute):
    start_time = time.time()
    
    if debug:
        print(f"Starting processing for directory: {directory}")
    cache_file = get_cache_file_name(directory, default_date)
    cache = load_cache(cache_file)
    metadata_info = save_metadata(directory, cache, debug)
    save_cache(metadata_info, cache_file)
    guesses = guess_dates(metadata_info, default_date, debug)
    
    modified_files_count = 0
    
    for file_path, guess_info in guesses.items():
        date = guess_info['date']
        reason = guess_info['reason']
        print(f"File: {file_path} - Guessed Date: {date} - Reason: {reason}")
        # Generate the exiftool command to set the DateTimeOriginal and add a comment
        command = [
            'exiftool',
            f'-DateTimeOriginal="{date}"',
            f'-UserComment="{reason}"',
            f'"{file_path}"'
        ]
        if execute:
            if debug:
                print(f"Executing: {' '.join(command)}")
            result = subprocess.run(' '.join(command), shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            if result.returncode != 0:
                print(f"Error executing command for file {file_path}: {result.stderr}")
                break
            else:
                print(f"Success: {result.stdout}")
                modified_files_count += 1
        else:
            print(f"Command to set guessed date: {' '.join(command)}")
            modified_files_count += 1

    end_time = time.time()
    elapsed_time = end_time - start_time

    print(f"Total files processed: {len(guesses)}")
    print(f"Total files modified: {modified_files_count}")
    print(f"Time taken: {elapsed_time:.2f} seconds")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Fix EXIF metadata for images in a directory.")
    parser.add_argument("directory", help="Path to the directory containing images.")
    parser.add_argument("--default-date", help="Default date to use for guessing (format: YYYY:MM:DD HH:MM:SS). If not provided, the script will use the beginning of the year of the first file that has an EXIF date.")
    parser.add_argument("--debug", action="store_true", help="Enable debug logging.")
    parser.add_argument("--execute", action="store_true", help="Execute the exiftool commands.")
    
    args = parser.parse_args()
    main(args.directory, args.default_date, args.debug, args.execute)

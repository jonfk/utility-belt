#!/usr/bin/env python3

import os
import subprocess
from collections import defaultdict

def get_metadata(file_path):
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

def save_metadata(directory):
    metadata_info = {}
    for filename in os.listdir(directory):
        file_path = os.path.join(directory, filename)
        if os.path.isfile(file_path):
            metadata_info[filename] = get_metadata(file_path)
    return metadata_info

def guess_dates(metadata_info):
    # Separate files with and without DateTimeOriginal
    with_date = {}
    without_date = {}
    
    for filename, metadata in metadata_info.items():
        if 'Date/Time Original' in metadata:
            with_date[filename] = metadata['Date/Time Original']
        else:
            without_date[filename] = metadata
    
    # Guess dates based on filenames
    guesses = {}
    sorted_files = sorted(with_date.keys())
    
    for filename in sorted(without_date.keys()):
        closest_before = None
        closest_after = None
        
        for ref_file in sorted_files:
            if ref_file < filename:
                closest_before = ref_file
            elif ref_file > filename and closest_after is None:
                closest_after = ref_file
        
        if closest_before and closest_after:
            guesses[filename] = (f"Between {with_date[closest_before]} (based on {closest_before}) and {with_date[closest_after]} (based on {closest_after}) "
                                 "based on filename sequence.")
        elif closest_before:
            guesses[filename] = (f"After {with_date[closest_before]} (based on {closest_before}) based on filename sequence.")
        elif closest_after:
            guesses[filename] = (f"Before {with_date[closest_after]} (based on {closest_after}) based on filename sequence.")
        else:
            guesses[filename] = "Beginning of the year guessed."
    
    return guesses

def main(directory):
    metadata_info = save_metadata(directory)
    guesses = guess_dates(metadata_info)
    
    for filename, guess in guesses.items():
        print(f"File: {filename} - Guessed Date: {guess}")

if __name__ == "__main__":
    directory_path = input("Enter the path to the directory: ")
    main(directory_path)

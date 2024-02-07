#!/bin/bash

# Check if an input file was provided
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <path_to_srt_file>"
    exit 1
fi

input_file="$1"
backup_file="${input_file}.backup"
temp_file="${input_file}.tmp"

# Make a backup of the original file
backup_dir="~/.whisper-hallucinations-backups"
mkdir -p backup_dir
cp "$input_file" "$backup_dir/$backup_file"

# Patterns to filter (add patterns here as needed)
patterns=(
    "Please subscribe to our channel"
    "Please subscribe"
    "Please click"
    "Thank you for watching"
    "Thank you for your viewing"
    "Touhou"
    "Translated by"
    "Subtitles by"
    "See you in the next video"
)

# Create a grep pattern string
grep_pattern=$(printf "|%s" "${patterns[@]}")
grep_pattern="${grep_pattern:1}" # remove the leading "|"

# Filter the file and adjust sequence numbers
awk -v pat="$grep_pattern" '
BEGIN { RS=""; FS="\n"; IGNORECASE=1; seq_num=1 }
{
    print_output=1;
    for(i=2; i<=NF; i++) { # Start from 2 to skip the sequence number
        if ($i ~ pat) {
            print_output=0;
            printf "Filtered out:\n%s\n\n", $0; # Output filtered entries
            break;
        }
    }
    if (print_output) {
        # Adjust sequence number and output to temporary file
        printf "%d\n", seq_num > "'"$temp_file"'";
        for(i=2; i<=NF; i++) { # Output the rest of the entry
            printf "%s\n", $i > "'"$temp_file"'";
        }
        printf "\n" > "'"$temp_file"'"; # Add a blank line after each entry
        seq_num++; # Increment sequence number for the next entry
    }
} END {}' "$backup_file"

# Move the temporary file to the original file
mv "$temp_file" "$input_file"

echo "Filtering complete. Original file updated with continuous sequence numbers. Backup created as $backup_file."

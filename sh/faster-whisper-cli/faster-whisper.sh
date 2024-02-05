#!/bin/bash

#set -x
set -e

# Check if at least one argument is given (the file path)
if [ $# -lt 1 ]; then
    echo "Usage: $0 <file_path> [--convert-to-mp3]"
    exit 1
fi

FILE_PATH="$1"
CONVERT_TO_MP3=false
echo "File path: $FILE_PATH"
echo "Convert to MP3: $CONVERT_TO_MP3"

# Check for the convert to mp3 flag
if [ "$2" == "--convert-to-mp3" ]; then
    CONVERT_TO_MP3=true
fi

# Extract filename without extension and directory path
FILENAME=$(basename -- "$FILE_PATH")
FILEDIR=$(dirname -- "$FILE_PATH")
FILENAME_NO_EXT="${FILENAME%.*}"

echo "Filename: $FILENAME"
echo "File directory: $FILEDIR"
echo "Filename without extension: $FILENAME_NO_EXT"

# Check if the output .srt file already exists
EXPECTED_SRT_PATH="${FILEDIR}/${FILENAME_NO_EXT}.srt"
if [ -f "$EXPECTED_SRT_PATH" ]; then
    echo "Error: Output .srt file $EXPECTED_SRT_PATH already exists. Please remove it before rerunning this script."
    exit 1
fi

# Generate hash of the filename
HASH=$(echo -n "$FILENAME_NO_EXT" | md5sum | cut -d' ' -f1)
echo "Hash of filename: $HASH"

# Create output directory
OUTPUT_DIR="${FILEDIR}/${HASH}_whisper"
mkdir -p "$OUTPUT_DIR"
echo "Output directory: $OUTPUT_DIR"

INPUT_FOR_WHISPER="$FILENAME" # Default to original file name for whisper command

# Convert to MP3 if the flag is set and adjust INPUT_FOR_WHISPER
if [ "$CONVERT_TO_MP3" = true ]; then
    OUTPUT_MP3="${OUTPUT_DIR}/${FILENAME_NO_EXT}.mp3"
    ffmpeg -i "$FILE_PATH" -q:a 0 -map a "$OUTPUT_MP3"
    INPUT_FOR_WHISPER="${FILENAME_NO_EXT}.mp3" # Update to converted MP3 filename for whisper
    echo "Converted MP3 file: $OUTPUT_MP3"
fi

echo "Input for Whisper: $INPUT_FOR_WHISPER"

# Run the whisper command with adjusted bindings and output directory
sudo docker run --rm --gpus all \
--mount type=bind,source=/srv/docker/whisper-webui/whisper,target=/root/.cache/whisper \
--mount type=bind,source=/srv/docker/whisper-webui/huggingface,target=/root/.cache/huggingface \
--mount type=bind,source="${FILEDIR}",target=/app/data \
faster-whisper-webui:1 \
cli.py --whisper_implementation faster-whisper --model large-v3 --task translate --language Japanese --auto_parallel True --vad silero-vad \
--output_dir /app/data/"${HASH}_whisper" /app/data/"$INPUT_FOR_WHISPER"

# Find the .srt file in the output directory
SRT_FILE=$(find "$OUTPUT_DIR" -type f -name "*.srt")

# Check if the .srt file exists
if [ -z "$SRT_FILE" ]; then
    echo "No transcript file found."
    exit 2 # Exit with error if no transcript file is found
else
    # Move the .srt file to the same directory as the input file and rename it to match the original file name with .srt extension
    NEW_SRT_PATH="${FILEDIR}/${FILENAME_NO_EXT}.srt"
    mv "$SRT_FILE" "$NEW_SRT_PATH"
    echo "Transcript file moved and renamed to $NEW_SRT_PATH"
    
    # Clean up the output directory by deleting it
    rm -rf "$OUTPUT_DIR"
    echo "Output directory ${OUTPUT_DIR} deleted."
fi

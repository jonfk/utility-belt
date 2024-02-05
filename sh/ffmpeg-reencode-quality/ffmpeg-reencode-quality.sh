#!/bin/bash

# Check if an input file was provided
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <input_file>"
    exit 1
fi

input_file="$1"
filename="${input_file%.*}"
extension="${input_file##*.}"
output_file="${filename}_reenc.${extension}"

# Use ffmpeg to re-encode the video
ffmpeg -i "$input_file" -vcodec libx264 -crf 21 "$output_file"

# Echo the name of the output file
echo "$output_file was reencoded"

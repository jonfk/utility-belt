#!/bin/sh

set -e

if [ $# -eq 0 ]; then
    echo "Usage: $0 <input.d2>" >&2
    exit 1
fi

input_file="$1"
output_file="${input_file%.*}.svg"

D2_LAYOUT=tala d2 "$input_file" "$output_file" || { echo "Error: D2 processing failed" >&2; exit 1; }
sed -i'' -e 's|<text[^>]*>UNLICENSED COPY</text>||g' "$output_file" || { echo "Error: sed processing failed" >&2; exit 1; }
echo "Processed $output_file"

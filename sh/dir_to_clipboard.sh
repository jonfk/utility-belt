#!/bin/bash

usage() {
    echo "Usage: $0 [-h] [-e exclude_pattern] [-i include_pattern]"
    echo "  -h: Display this help message"
    echo "  -e exclude_pattern: Exclude files matching this pattern"
    echo "  -i include_pattern: Include only files matching this pattern"
    exit 1
}

exclude_pattern=""
include_pattern=""

while getopts "he:i:" opt; do
    case ${opt} in
        h )
            usage
            ;;
        e )
            exclude_pattern=$OPTARG
            ;;
        i )
            include_pattern=$OPTARG
            ;;
        \? )
            usage
            ;;
    esac
done

if [[ "$OSTYPE" == "darwin"* ]]; then
    clipboard_cmd="pbcopy"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    if ! command -v xclip &> /dev/null; then
        echo "Error: xclip is not installed. Please install it using your package manager."
        exit 1
    fi
    clipboard_cmd="xclip -selection clipboard"
else
    echo "Unsupported operating system"
    exit 1
fi

find_cmd="find . -type f"
if [ -n "$exclude_pattern" ]; then
    find_cmd="$find_cmd -not -path '$exclude_pattern'"
fi
if [ -n "$include_pattern" ]; then
    find_cmd="$find_cmd -path '$include_pattern'"
fi

$find_cmd -print0 | xargs -0 -I {} bash -c "echo -e '\n--- {} ---\n' && cat '{}'" | $clipboard_cmd

echo "File contents copied to clipboard"

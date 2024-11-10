#!/bin/bash

# Configuration
GITHUB_USER="jonfk"
AUTH_KEYS="$HOME/.ssh/authorized_keys"
GITHUB_COMMENT="# GitHub: $GITHUB_USER"
TEMP_FILE=$(mktemp)

# Function to fetch GitHub keys
fetch_github_keys() {
    if ! curl -sf "https://github.com/$GITHUB_USER.keys" > "$TEMP_FILE"; then
        echo "Error: Failed to fetch GitHub keys for user $GITHUB_USER"
        rm "$TEMP_FILE"
        exit 1
    fi

    if [ ! -s "$TEMP_FILE" ]; then
        echo "Error: No SSH keys found for GitHub user $GITHUB_USER"
        rm "$TEMP_FILE"
        exit 1
    fi
}

# Function to ensure authorized_keys file exists
ensure_auth_keys() {
    if [ ! -f "$AUTH_KEYS" ]; then
        mkdir -p "$(dirname "$AUTH_KEYS")"
        touch "$AUTH_KEYS"
        chmod 600 "$AUTH_KEYS"
    fi
}

# Function to create a backup of authorized_keys
backup_auth_keys() {
    cp "$AUTH_KEYS" "$AUTH_KEYS.backup.$(date +%Y%m%d_%H%M%S)"
}

# Function to normalize a key by removing comments and extra spaces
normalize_key() {
    echo "$1" | awk '{print $1, $2}' | tr -s ' '
}

# Function to get key type and data (first two fields)
get_key_data() {
    echo "$1" | awk '{print $1, $2}'
}

# Main sync function
sync_keys() {
    local temp_output=$(mktemp)
    local seen_keys=()
    local github_keys=()
    
    # Read GitHub keys into array and normalize them
    while IFS= read -r key || [[ -n "$key" ]]; do
        if [[ -n "$key" ]]; then
            github_keys+=("$(normalize_key "$key")")
        fi
    done < "$TEMP_FILE"

    # Process existing authorized_keys
    while IFS= read -r line || [[ -n "$line" ]]; do
        if [[ -n "$line" && "$line" != \#* ]]; then
            local normalized_key=$(normalize_key "$line")
            local key_data=$(get_key_data "$line")
            local is_duplicate=0
            local is_github_key=0

            # Check if this key is in GitHub keys
            for github_key in "${github_keys[@]}"; do
                if [[ "$normalized_key" == "$github_key" ]]; then
                    is_github_key=1
                    break
                fi
            done

            # Check if we've seen this key before
            for seen_key in "${seen_keys[@]}"; do
                if [[ "$normalized_key" == "$seen_key" ]]; then
                    is_duplicate=1
                    break
                fi
            done

            # Keep the key if it's not a duplicate and not from GitHub
            if [[ $is_duplicate -eq 0 && $is_github_key -eq 0 ]]; then
                echo "$line" >> "$temp_output"
                seen_keys+=("$normalized_key")
            fi
        elif [[ -n "$line" ]]; then
            # Preserve comments that aren't GitHub markers
            if [[ "$line" != *"$GITHUB_COMMENT"* ]]; then
                echo "$line" >> "$temp_output"
            fi
        fi
    done < "$AUTH_KEYS"

    # Add current GitHub keys with comment
    for github_key in "${github_keys[@]}"; do
        local is_duplicate=0
        for seen_key in "${seen_keys[@]}"; do
            if [[ "$github_key" == "$seen_key" ]]; then
                is_duplicate=1
                break
            fi
        done

        if [[ $is_duplicate -eq 0 ]]; then
            echo "$github_key $GITHUB_COMMENT" >> "$temp_output"
            seen_keys+=("$github_key")
        fi
    done

    # Replace authorized_keys with new content
    mv "$temp_output" "$AUTH_KEYS"
    chmod 600 "$AUTH_KEYS"
}

# Main execution
echo "Starting GitHub SSH key sync for user: $GITHUB_USER"

# Fetch GitHub keys
echo "Fetching GitHub keys..."
fetch_github_keys

# Ensure authorized_keys exists
ensure_auth_keys

# Create backup
echo "Creating backup of authorized_keys..."
backup_auth_keys

# Sync keys
echo "Syncing and deduplicating keys..."
sync_keys

# Cleanup
rm "$TEMP_FILE"

echo "Sync completed successfully!"
echo "Backup created at: $AUTH_KEYS.backup.$(date +%Y%m%d_%H%M%S)"
echo "Current authorized_keys content:"
echo "----------------------------------------"
cat "$AUTH_KEYS"

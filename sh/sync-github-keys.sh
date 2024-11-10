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

# Main sync function
sync_keys() {
    local temp_output=$(mktemp)
    
    # Process existing authorized_keys, keeping non-GitHub keys
    while IFS= read -r line || [[ -n "$line" ]]; do
        if [[ "$line" != *"$GITHUB_COMMENT"* ]] && [[ -n "$line" ]]; then
            echo "$line" >> "$temp_output"
        fi
    done < "$AUTH_KEYS"

    # Add current GitHub keys with comment
    while IFS= read -r key || [[ -n "$key" ]]; do
        if [[ -n "$key" ]]; then
            echo "$key $GITHUB_COMMENT" >> "$temp_output"
        fi
    done < "$TEMP_FILE"

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
echo "Syncing keys..."
sync_keys

# Cleanup
rm "$TEMP_FILE"

echo "Sync completed successfully!"
echo "Backup created at: $AUTH_KEYS.backup.$(date +%Y%m%d_%H%M%S)"
echo "Current authorized_keys content:"
echo "----------------------------------------"
cat "$AUTH_KEYS"

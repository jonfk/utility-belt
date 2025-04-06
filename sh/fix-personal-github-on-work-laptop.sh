#!/bin/sh

# This script searches for all Git repositories under the current directory
# and updates the remote URL for repositories that use GitHub.
# Specifically, it:
#   1. Finds all .git directories recursively
#   2. For each repository that has a GitHub remote URL:
#   3. Replaces 'github.com' with 'github-jonfk' in the origin URL
#   4. This effectively redirects all GitHub repositories to use an alternate
#      GitHub domain or mirror (github-jonfk)
# 
# Purpose:
#   This script addresses an SSH authentication issue when using multiple GitHub accounts.
#   The problem occurs because ssh-agent overrides Git's directory-based identity configuration.
#   Even though Git identities are properly separated by directories, ssh-agent ignores this
#   and selects SSH keys based solely on the hostname in the remote URL (github.com).
#   This causes personal repositories to incorrectly authenticate with a work account.
#   By changing URLs to use 'github-jonfk', repositories will match the correct host entry
#   in the SSH config, forcing ssh-agent to use the intended SSH key.
# 
# See:
# - https://github.com/jonfk/swe-notes/blob/main/git/multiple-git-profiles.md
# - https://github.com/jonfk/swe-notes/blob/main/git/specify-ssh-key-by-host.md

find . -type d -name ".git" -print0 | while IFS= read -r -d '' git_dir; do
    repo_dir="$(dirname "$git_dir")"
    cd "$repo_dir" || continue
    
    # Check if remote URL contains github.com
    if git remote -v | grep -q "github.com"; then
        echo "Updating repository: $repo_dir"
        
        # Get current remote information
        old_url=$(git remote get-url origin 2>/dev/null)
        
        # Check if we have a URL to change
        if [ -n "$old_url" ] && [[ "$old_url" == *"github.com"* ]]; then
            # Create new URL by replacing github.com with github-jonfk
            new_url="${old_url/github.com/github-jonfk}"
            
            echo "  Old URL: $old_url"
            echo "  New URL: $new_url"
            
            # Update the remote URL
            git remote set-url origin "$new_url"
            echo "✅ Remote URL updated"
        else
            echo "! No valid GitHub URL found for origin"
        fi
    fi
    
    # Return to the original directory
    cd - > /dev/null
done

# List all available recipes (default)
default:
    @just --list

# Install git-smart-commit using uv
install-git-smart-commit:
    cd python/git-smart-commit && uv tool install --reinstall .

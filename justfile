# List all available recipes (default)
default:
    @just --list

# Install git-smart-commit using uv
install-git-smart-commit:
    cd python/git-smart-commit && uv tool install --reinstall .

# Install start-ssh-proxy script
install-start-ssh-proxy:
    mkdir -p ~/.local/bin
    ln -sf $(pwd)/sh/start-ssh-proxy.sh ~/.local/bin/start-ssh-proxy

# Install sync-github-keys script
install-sync-github-keys:
    mkdir -p ~/.local/bin
    ln -sf $(pwd)/sh/sync-github-keys.sh ~/.local/bin/sync-github-keys

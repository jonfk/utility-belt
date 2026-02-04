# List all available recipes (default)
default:
    @just --choose

# Install git-smart-commit using uv
install-git-smart-commit:
    cd python/git-smart-commit && uv tool install --reinstall .

# Install git-worktree-utils using uv
install-git-worktree-utils:
    cd python/git-worktree-utils && uv tool install --reinstall .

# Install prune-openapi using uv
install-prune-openapi:
    cd python/prune-openapi && uv tool install --reinstall .

# Install start-ssh-proxy script
install-start-ssh-proxy:
    mkdir -p ~/.local/bin
    ln -sf $(pwd)/sh/start-ssh-proxy.sh ~/.local/bin/start-ssh-proxy

# Install sync-github-keys script
install-sync-github-keys:
    mkdir -p ~/.local/bin
    ln -sf $(pwd)/sh/sync-github-keys.sh ~/.local/bin/sync-github-keys

# Install organize-files-into-dirs-by-date script
install-organize-files-into-dirs-by-date:
    mkdir -p ~/.local/bin
    ln -sf $(pwd)/python/organize-files-into-dirs-by-date.py ~/.local/bin/organize-files-into-dirs-by-date

# Install beautiful-mermaid CLI
install-beautiful-mermaid:
    cd ts/beautiful-mermaid && bun install && bun run build
    mkdir -p ~/.local/bin
    ln -sf $(pwd)/ts/beautiful-mermaid/dist/beautiful-mermaid ~/.local/bin/beautiful-mermaid

# Install utility-belt manager script
install-utility-belt:
    mkdir -p ~/.local/bin
    chmod +x $(pwd)/sh/utility-belt.sh
    ln -sf $(pwd)/sh/utility-belt.sh ~/.local/bin/utility-belt

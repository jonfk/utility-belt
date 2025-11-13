# Git Smart Worktree — Functional Specification

## 1. Purpose & Scope
- Build a Python CLI (`git-smart-worktree`) that standardizes how git worktrees are created, listed, and removed for any repository based solely on generic git metadata.
- Focus on managing a mirrored "admin" clone plus organized worktree directories under user-defined roots, enforcing the layout described in `README.md`.
- Out of scope: host-specific API integrations (e.g., GitHub REST/GraphQL), PR automation, and advanced config stores beyond environment variables and hard-coded defaults.

## 2. Environment & Configuration
- **Required env vars** (validated before command execution):
  - `GIT_WORKTREE_ADMIN_ROOT`: root folder storing the canonical admin clone structure `<host>/<owner>/<repo>`.
  - `GIT_WORKTREE_ROOT`: root folder for worktree directories `<host>/<owner>/<repo>/<worktree-name>`.
- CLI exits with actionable messaging if either variable is unset or points to a non-writable path.
- **Repository detection**:
  - Default: infer repo from current working directory via `git rev-parse --show-toplevel`.
  - Global flag `--repo PATH` lets users target another repo; path must resolve to a git working tree.
- **Remote discovery** uses `git remote get-url origin` to parse `<host>/<owner>/<repo>` supporting HTTPS and SSH syntax. `.git` suffixes are stripped.
- **Default branch** detection via `git symbolic-ref refs/remotes/origin/HEAD`; fallback order `main` → `master` if symbolic ref missing.

## 3. Directory Layout Policy
- Admin clone lives at `GIT_WORKTREE_ADMIN_ROOT/<host>/<owner>/<repo>` (non-bare, `--no-checkout`). Created lazily if absent by cloning the resolved origin URL once.
- Worktrees live under `GIT_WORKTREE_ROOT/<host>/<owner>/<repo>/<worktree-name>`.
- Worktree names are user supplied and slugified by replacing `/` with `__`, spaces with `-`, and non-alphanumeric characters with `-`.
- CLI ensures parent directories exist before invoking `git worktree add`.

## 4. Dependencies & Tooling
- Python ≥ 3.13 (per `pyproject.toml`).
- Libraries:
  - `typer[all]` (CLI ergonomics, help text, rich output). 
  - `InquirerPy` for interactive prompts with built-in fuzzy search; optional fallback to non-interactive mode when all arguments supplied.
  - `rich` (optional) for table output and colored messaging.
- Git CLI is invoked via subprocess; no additional VCS libraries required.

## 5. CLI Surface
- Root command: `git-smart-worktree`.
- **Global options**: `--repo PATH`, `--verbose`, `--version`.
- **Commands**:
  - `ls [--all] [--json]`
    - Lists known worktrees for the resolved repo. Default view: table with columns `name`, `branch`, `path`, `status`.
    - `--all` scans filesystem directories even if git no longer tracks them; otherwise rely on `git worktree list --porcelain`.
    - `--json` outputs machine-readable JSON array.
  - `add [WORKTREE_NAME] [BRANCH] [--from START] [--track REMOTE]`
    - Adds a worktree rooted at `<worktree-name>`; arguments optional.
    - If `BRANCH` does not exist, create it (optionally from `START`, defaulting to repo default branch) before `git worktree add`.
    - Non-interactive when all required params are provided; otherwise opens guided prompts (Section 6).
  - `rm [PATH] [--force] [--select]`
    - Removes one or more worktrees using `git worktree remove`.
    - If `PATH` omitted, `--select` enables interactive fuzzy selection; otherwise listing is shown with index numbers for typed selection.
    - `--force` passes through to git for unclean worktrees.

## 6. Interactive & Fuzzy UX
- Triggered whenever `add` or `rm` lack enough arguments or when users pass `--select` explicitly.
- Prompts use `InquirerPy` fuzzy finder:
  - **Worktree name input**: free-form text box validated to ensure non-empty, slash-free values before slugifying for the filesystem.
  - **Branch selection**: merges local (`git branch --format`) and remote (`git branch -r`) names; default branch is pinned to the top. Options: choose existing or "Create new branch…" to type a name.
  - **Starting point selection** (only when creating a new branch): options include default branch, any existing branch/tag, or manual ref input.
  - **Worktree removal selection**: entries display `name (branch) · path`. Supports multi-select if we want to extend later; MVP removes one at a time.
- Users can bypass prompts entirely by supplying all positional arguments.

## 7. Git Operations & Edge Cases
- **Admin clone bootstrap**:
  - If missing, run `git clone --no-checkout <origin> <admin_path>`.
  - Subsequent commands use `--git-dir <admin_path>/.git` and `--work-tree <admin_path>` to keep operations scoped to the admin clone when adding/removing worktrees.
- **Worktree creation flow**:
  1. Ensure admin clone exists and fetch latest refs (`git fetch origin --prune`).
  2. If target branch exists (local or remote), use `git worktree add <target_path> <branch>`.
  3. If new branch, create via `git worktree add <target_path> --checkout -b <branch> <start-point>`.
- **Listing**: parse `git worktree list --porcelain` from admin clone to stay consistent. Each entry includes `worktree`, `branch`, `locked`, `prunable` fields.
- **Removal**: call `git worktree remove [--force] <path>` in admin clone context, then delete empty directories left behind in `GIT_WORKTREE_ROOT`.
- **Slug collisions**: if two worktree names map to the same slug (e.g., `feature/foo` and `feature__foo`), warn the user and refuse creation unless they choose a different name.

## 8. Error Handling & Messaging
- Centralized error classes: `MissingEnvError`, `GitError`, `ValidationError`, `UserAbort`.
- Human-friendly guidance for common issues:
  - Missing env vars → show export examples.
  - Not inside git repo → suggest `--repo` flag or `cd` instructions.
  - Git command failures → display succinct stderr plus tip (`git fetch origin --prune`).
- Non-zero exit codes map to failure categories (1=usage/config, 2=git failure, 130=user aborted).

## 9. Architecture Overview
- Package layout (under `git_smart_worktree/`):
  - `cli.py`: Typer entrypoint wiring subcommands.
  - `config.py`: env validation, repo context resolution, path builders.
  - `git.py`: subprocess helpers wrapping git commands and parsing outputs.
  - `worktrees.py`: business logic for listing, adding, removing worktrees using config + git helpers.
  - `interactive.py`: prompt utilities (fuzzy select, confirm, text input) abstracted for easy mocking.
  - `models.py`: dataclasses (`RepoContext`, `Paths`, `WorktreeEntry`).
  - `exceptions.py`: shared error types.
- Tests: use `pytest` with fixtures mocking subprocess responses; Typer commands covered via `CliRunner`.

## 10. Future Extensions (Non-blocking)
- Optional tagging or grouping metadata stored alongside worktree directories.
- Optional integration with external fuzzy tools (`fzf`) for users who prefer native binaries.
- Multi-select removal and batch add flows.
- Metrics/logging hooks, e.g., `--dry-run` preview mode.

# git-smart-branch-delete

Interactive local git branch deletion using `fzf`.

The script lists local branches in the current repository (excluding protected
branches), opens an `fzf` multi-select picker, then deletes the selected
branches using `git branch --delete` (or `--force` when requested).

## Requirements

- `git`
- `fzf`

## Usage

Run from anywhere inside a git repo:

```bash
python3 /path/to/utility-belt/python/git-smart-branch-delete/main.py
```

Or from this directory:

```bash
python3 main.py
```

With `uv`:

```bash
uv run python main.py
```

## Behavior

- Protected by default: `main`, `master`, and the currently checked out branch.
- You can add more protected branches with `--protect BRANCH` (repeatable).
- `fzf` is run with multi-select enabled (TAB to toggle selections).
- After selection, the script asks for confirmation unless `--yes` is provided.
  The confirmation prompt defaults to **Yes**.
- Deletion uses `git branch --delete`. If a branch is unmerged (or otherwise
  cannot be deleted), git will refuse unless you pass `--force`.

## Options

- `--dry-run`: Print the delete commands without deleting anything.
- `--force`: Force delete branches (equivalent to `git branch -D`).
- `--yes`: Skip the confirmation prompt after selection.
- `--protect BRANCH`: Protect a branch from deletion (repeatable).

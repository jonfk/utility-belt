I would like to create a cli application that standardizes the creation of git worktrees from a given repository. It should be aware of the current directory's git repo.
I envision the directory tree for storing the worktree and admin repo as follows:

```
  ~/.gitsrc/github.com/acme/payment-service
  ~/worktrees/github.com/acme/payment-service/main
  ~/worktrees/github.com/acme/payment-service/user-payments
  ~/worktrees/github.com/acme/payment-service/pr-842
  ~/worktrees/github.com/acme/payment-service/release-1-8-x
```
There are 2 root directories to store worktree related things: the git worktree admin root and the git worktree root
- The admin clones are stored at GIT_WORKTREE_ADMIN_ROOT/<host>/<owner>/<repo> (non-bare, --no-checkout).
- The git worktrees are stored at GIT_WORKTREE_ROOT/<host>/<owner>/<repo>/<worktree-name>.

I want the following functionality:
- The program should be aware of the git repository based on the working directory, but a global flag can override what git repository we want to work on worktrees for.
- The program should error out if GIT_WORKTREE_ADMIN_ROOT and GIT_WORKTREE_ROOT aren't defined.
- ls command: I can list the worktrees for the repository I am currently in.
- rm command: I can delete worktrees. This should take the path to a worktree as argument. If none is provided, it should open an interactive mode where worktrees for the repository in the current path are shown and deleted on selection. It uses git worktree remove under the hood.
- add command: I can add a git worktree for the current directory's repository. It should take a user-supplied worktree name, a branch name and an optional starting point. If those are not provided and add is called without these arguments, it should open in an interactive mode where these are prompted to the user to select or add values for them.
    - worktree-name: Arbitrary label for the directory that will be created under the repository-scoped worktree root. It must be unique and filesystem safe.
    - branch: the branch to checkout or create. The default branch main/master should be at the top of the list.
    - from: the starting point of a new branch if created. Can be omitted if checking out an existing branch. If empty or omitted on creating a new branch, it should be the default branch (main/master). This can also be a selector from existing branches.

## Quick start

```bash
export GIT_WORKTREE_ADMIN_ROOT=~/worktree-admin
export GIT_WORKTREE_ROOT=~/worktrees

# Install dependencies (uv, pip, etc.)
pip install -e .

# Explore commands
git-worktree-utils --help
```

Run the CLI inside any git repository. Use `--repo /path/to/repo` to operate on another project without changing directories.

## Commands

| Command | Description |
| --- | --- |
| `git-worktree-utils ls [--json]` | Lists worktrees for the detected repo using the admin clone (table or JSON). |
| `git-worktree-utils add [WORKTREE_NAME] [BRANCH] [--from REF] [--track REMOTE]` | Adds a worktree. Missing arguments trigger interactive prompts for name/branch/start. |
| `git-worktree-utils rm [PATH] [--force] [--select]` | Removes a worktree by path or via an interactive selector that lists existing entries. |

Both `GIT_WORKTREE_ADMIN_ROOT` and `GIT_WORKTREE_ROOT` must point to writable directories. Admin clones live at `GIT_WORKTREE_ADMIN_ROOT/<host>/<owner>/<repo>` and worktrees at `GIT_WORKTREE_ROOT/<host>/<owner>/<repo>/<worktree-name>`.

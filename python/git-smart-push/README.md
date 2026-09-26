# git-smart-push

Runs `git push`, scans the push output for a GitHub pull-request creation URL
containing `/pull/new`, and optionally offers to open that URL.

The open prompt defaults to **Yes** (`[Y/n]`).

## Requirements

- `git`
- Optional: `GIT_OPEN_CMD` naming an executable to open URLs. This is a single
  executable name or path, not a shell command with arguments.

The Nix package includes Python and Git. Build it from the repository root with
`nix build .#git-smart-push`, or run it with `nix run .#git-smart-push -- origin HEAD`.

## Usage

From inside a git repository:

```bash
git-smart-push
```

Forward any normal `git push` args:

```bash
git-smart-push origin HEAD
git-smart-push --force-with-lease
```

## Behavior

- Executes `git push` with the provided arguments.
- Prints the full push output.
- Detects the first URL containing `/pull/new` in that output.
- Prints the detected URL. When `GIT_OPEN_CMD` is unset, empty, or unavailable,
  does not prompt or launch a browser.
- Otherwise prompts: `Open this URL in browser? [Y/n]` and, if accepted, passes
  the URL to that executable. For example, `GIT_OPEN_CMD=open git-smart-push`
  uses the macOS browser opener.
- Preserves Git's exit status even if opening the browser fails.

## Development

Run behavioral tests with `uv run python -m unittest discover -s tests -v`.
From the repository root, `nix flake check` also verifies that the packaged
command can push to a local repository without Git in the caller's `PATH`.

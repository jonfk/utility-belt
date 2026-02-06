# git-smart-push

Runs `git push`, scans the push output for a GitHub pull-request creation URL
containing `/pull/new`, and offers to open that URL with `open <url>`.

The open prompt defaults to **Yes** (`[Y/n]`).

## Requirements

- `git`
- `open` (macOS command used to launch the browser)

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
- Prompts: `Open this URL in browser? [Y/n]`.
- If accepted, runs `open <url>`.

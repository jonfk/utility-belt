# tmuxp-lite Design

Status: Draft

## Summary

`tmuxp-lite` is a small Python CLI for saving and restoring multiple tmux
sessions from one compact YAML file. It targets the subset of tmuxp behavior
that matters for lightweight personal sessions: session names, ordered windows,
working directories, the foreground command or process as a non-executing prompt
hint, and coarse pane orientation.

The proposed design stores all managed sessions in a single human-editable file
under the user's config directory. The tool captures all current tmux sessions
by default and restores all saved sessions by default, while still allowing
specific session names to be passed. It intentionally avoids tmuxp's full layout
fidelity, hooks, shell setup model, compatibility concerns, and per-session
files.

The main trade-off is accepting imperfect restore fidelity in exchange for a
quiet, inspectable workflow that is easy to adjust manually after restore.

## Problem

tmuxp is useful for restoring tmux workspaces, but it stores sessions as
separate files. That becomes noisy when many sessions are small and similar. The
current need is narrower than tmuxp's feature set:

- preserve enough information to recreate useful tmux sessions after reloads
- keep all small sessions in one place
- avoid modeling exact pane geometry
- allow manual adjustment after restore

The goal is not to create another comprehensive tmux workspace manager or a
tmuxp-compatible format. The tool should be a focused capture and restore helper
for personal tmux usage.

## Goals

- Save all managed sessions in one YAML file.
- Capture all currently running tmux sessions by default.
- Restore all saved sessions by default, or restore selected named sessions.
- Represent sessions, ordered windows, working directories, shell command or
  foreground process hints, and simple pane orientation.
- Preserve configured window order during restore.
- Keep pane restore behavior simple and predictable.
- Use Python with `uv`-managed packaging, following the repo's Python tooling
  convention.
- Install through the root `justfile` and `sh/utility-belt.sh` if promoted to
  an active utility.

## Non-Goals

- Exact tmux layout recreation.
- Full tmuxp compatibility.
- Importing or exporting tmuxp files.
- A daemon that continuously tracks tmux changes.
- Cross-host sync or remote tmux server management.
- Terminal emulator session management.
- Automatic recovery of terminal scrollback or shell history.

## Requirements and Decision Criteria

- Low config noise: many small sessions should remain comfortable in one file.
- Manual editability: the saved format should be readable and stable enough to
  edit by hand.
- Safe restore: restoring should not destroy live sessions unless explicitly
  requested.
- Capture transparency: the tool should make clear when a command could not be
  confidently inferred.
- Minimal dependencies: add dependencies only if they materially improve config
  parsing or CLI ergonomics.
- tmux-native behavior: use `tmux` commands and format strings instead of
  scraping human-formatted tmux output.

## Proposed User Model

The user manages a single config file such as:

```yaml
version: 1
sessions:
  utility-belt:
    root: ~/repos/utility-belt
    windows:
      - name: editor
        cwd: ~/repos/utility-belt
        command: nvim
      - name: shell
        cwd: ~/repos/utility-belt
      - name: server
        cwd: ~/repos/utility-belt/js/video-downloader-server
        command: npm run dev
  notes:
    root: ~/notes
    windows:
      - name: journal
        cwd: ~/notes
        command: nvim daily.md
```

Pane support stays deliberately coarse:

```yaml
  infra:
    root: ~/repos/infra
    windows:
      - name: ops
        panes:
          orientation: horizontal
          items:
            - cwd: ~/repos/infra
              command: htop
            - cwd: ~/repos/infra
```

Session names are the stable identity. Renaming a session is treated as removing
one saved session and adding another. Window order is the order in the YAML list
and should be preserved during restore.

If a window has no `panes`, it is restored as one pane using the window `cwd` or
session `root`. If it has panes, each pane item must include its own `cwd`; the
tool creates the initial pane from the first pane item and one split per
additional pane using either `horizontal` or `vertical`. It does not try to
preserve pane sizes or nested layouts.

The `command` field is a prompt hint, not an automatic command runner. Restore
types the configured text into the pane without pressing Enter. This makes
lossy captured process names visible while leaving execution under user control.

## Functional Design

### Commands

Initial commands:

- `tmuxp-lite capture`: inspect all sessions in the current tmux server and
  replace the config file.
- `tmuxp-lite restore [session...]`: restore named sessions, or all configured
  sessions if no name is provided.
- `tmuxp-lite list`: list saved session names from the config file.
- `tmuxp-lite edit`: open the config file in `$EDITOR`.

Useful later commands:

- `tmuxp-lite diff`: compare live tmux sessions with saved config.
- `tmuxp-lite prune`: remove saved sessions that are not live.

There is no `show` command in the first version. Opening the config file is good
enough for review, and keeping the command surface small is part of the design.

### Capture Behavior

Capture includes all live tmux sessions by default. It queries tmux using format
strings:

- `tmux list-sessions`
- `tmux list-windows -a`
- `tmux list-panes -a`

For each pane, capture should prefer tmux-provided fields such as session name,
window index, window name, pane index, current path, pane command, and active
pane metadata. The first version should treat `pane_current_command` as the best
available command hint, not as a perfect restart command.

For shells, capture can omit `command` and rely on tmux's default shell during
restore. For long-running processes, it can save the foreground process name as
a prompt for manual cleanup. Reconstructing the original command line is useful
but platform-sensitive, so it should be deferred unless the limitation becomes
painful in practice.

Capture writes the captured tmux state as the new file contents. There is no
merge mode in the first version. This keeps the command easy to reason about:
the saved file is a snapshot of the current tmux server at capture time.

### Restore Behavior

Restore should be conservative:

- If no session names are passed, restore all configured sessions.
- If one or more session names are passed, restore only those sessions.
- If a target session does not exist, create it.
- If a target session already exists, skip it by default.
- `--attach` attaches or switches the client to the restored session.
- `--kill-existing` explicitly kills and recreates a conflicting session.

Window restore:

- Create the session with the first configured window.
- Create remaining windows with configured names and directories in YAML order.
- Type configured command hints through tmux `send-keys -l` after pane creation
  without pressing Enter.

Pane restore:

- Create panes in listed order.
- Use each pane item's required `cwd`; pane-group windows do not also set a
  window-level `cwd`.
- Use one orientation for the window.
- Focus the first pane after creation.
- Do not attempt exact pane dimensions.

### Config Location

Default config path:

`$XDG_CONFIG_HOME/tmuxp-lite/sessions.yaml`

Fallback:

`~/.config/tmuxp-lite/sessions.yaml`

All commands should accept `--config PATH`.

### File Format

YAML is the selected format because it is pleasant for a manually edited
multi-session file and supports comments. Use a small dependency such as PyYAML
if needed.

The config should include a `version` field from the start so later schema
changes are explicit.

## Technical Design

### Packaging

Use a small `uv` package under `python/tmuxp-lite/`:

- `pyproject.toml`
- `tmuxp_lite/__main__.py`
- `tmuxp_lite/cli.py`
- `tmuxp_lite/config.py`
- `tmuxp_lite/models.py`
- `tmuxp_lite/tmux.py`
- `tmuxp_lite/capture.py`
- `tmuxp_lite/restore.py`
- `tests/`

Use Typer if command shape and help output benefit from it, matching existing
repo Python tools. Otherwise, `argparse` is enough and avoids a dependency.

### Core Types

Suggested model:

- `Config`: version and session map.
- `SessionSpec`: name, root, ordered windows.
- `WindowSpec`: name, cwd, command, panes. `cwd` and `command` apply only to
  single-pane windows.
- `PaneGroupSpec`: orientation and items.
- `PaneSpec`: required cwd and optional command hint.
- `LiveSession`, `LiveWindow`, `LivePane`: parsed tmux state.

The config model should preserve unknown future fields only if that can be done
without much complexity. Otherwise, reject unknown fields early with useful
errors.

### tmux Integration

Create one subprocess adapter around `tmux`:

- executes commands with consistent error handling
- emits machine-readable tmux format strings using a delimiter unlikely to
  appear in names or paths
- validates that tmux is installed and that a server is available for capture

The application layer should consume parsed live-state objects rather than raw
tmux output.

### Command Inference

There are three possible levels of command capture:

- shell-only: omit commands when the pane is a shell
- process-name: save `pane_current_command`
- full-command best effort: inspect OS process tables for command arguments

The proposed first version uses shell-only plus process-name capture. Captured
process names are restored only as prompt hints, not executed commands. Full
command-line capture can be added later behind a flag if it proves worth the
platform-specific complexity.

This means a captured `nvim` pane might restore with `nvim` sitting at the
prompt, not necessarily `nvim some/file.md`, and the user chooses whether to run
or edit it. That limitation is acceptable for the first version if the file
remains easy to edit.

## Alternatives Considered

### Continue using tmuxp

tmuxp already solves rich session restore and is battle-tested. It is rejected
because the per-session file model is the main source of friction, and the
desired behavior is much smaller than tmuxp's surface area.

### tmuxp Compatibility or Import

tmuxp compatibility could make migration easier, but this tool is intended as a
new, smaller workflow rather than an adapter around tmuxp's data model. Carrying
compatibility requirements would add complexity to a design whose main value is
being focused and easy to reason about.

### Shell Script

A shell script would be enough for simple restore commands, but capture,
normalization, validation, and YAML writing are easier to maintain in Python.

### Rust CLI

Rust would provide fast startup and strong typing, matching some other tools in
the repo. Python is preferred here because the workflow is text/config-heavy,
the implementation should stay script-like, and `uv` packaging is already used
for active Python utilities.

### SQLite Store

SQLite would make incremental updates and history easier, but the user-visible
problem is file noise, not query complexity. A single config file is simpler and
more inspectable.

### JSONC Config

JSONC would provide comments while avoiding YAML's implicit scalar behavior, but
Python's standard library does not include a JSONC parser. If a dependency is
needed anyway, YAML is the more natural format for this hand-edited nested
configuration.

## Trade-Offs and Risks

- Captured commands may be lossy because tmux exposes the current process name,
  not necessarily the original shell command line.
- YAML is pleasant to edit but can surprise users with implicit scalar parsing.
- Replace-only capture is simple, but manual edits for non-live sessions are
  lost if the user captures over the file.
- Sending command hints with `send-keys -l` leaves execution under user control
  but requires pressing Enter manually for commands the user actually wants to
  run.
- Restoring all sessions by default is convenient, but it can create more
  sessions than intended if the config file has grown stale.
- Coarse pane orientation intentionally loses layout detail.

## Open Questions

No blocking product questions remain for the first implementation. Details such
as exact CLI option names and error wording can be settled during implementation.

## Implementation Plan

1. Create the Python package skeleton with CLI entry point and config models.
2. Implement config load/save, schema validation, path expansion, `list`, and
   `edit`.
3. Implement tmux live-state querying for capture.
4. Implement replace-only `capture`.
5. Implement conservative `restore` with conflict detection and one-pane
   windows.
6. Add simple pane restore with horizontal/vertical split support.
7. Add focused tests for config parsing, capture writing, tmux output parsing,
   and restore planning.
8. Add install recipe to the root `justfile` and register the active program in
   `sh/utility-belt.sh` if the design is accepted.

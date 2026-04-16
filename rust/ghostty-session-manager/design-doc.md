# Ghostty Session Manager Design

## Overview

`ghostty-session-manager` is a Rust CLI for treating Ghostty windows as
project-scoped sessions. It uses Ghostty's AppleScript API as the control plane
for reading window state and performing actions such as focusing windows and
creating new ones.

The first version should optimize for simplicity:

- no background daemon
- JSON persistence
- one-shot CLI commands
- a terminal UI for interactive switching

## Key Decisions

### Language

Rust is the implementation language.

Reasons:

- fast startup for a frequently used local CLI
- strong fit for terminal applications
- good subprocess handling for `osascript`
- good long-term fit if this grows beyond a simple script

### Automation Boundary

Ghostty exposes AppleScript, not a native Rust API. The expensive boundary is
the automation call itself, not the host language runtime.

Design consequence:

- minimize AppleScript round trips
- fetch as much state as possible in a single script execution
- do ranking, search, persistence, and UI logic in Rust

### Persistence

State will be stored in a JSON file to start.

Reasons:

- easy to inspect manually
- easy to reset
- no schema migration burden in the first phase
- enough for MRU tracking and lightweight metadata

SQLite can be introduced later if the state model becomes more complex.

### Interactive UI

Interactive switching will use:

- `ratatui` for the terminal UI
- `frizbee` for fuzzy matching and ranking support

This avoids shelling out to `fzf` and keeps control of scoring and display
inside the application.

## Platform Constraints

- macOS only
- Ghostty AppleScript support must be enabled
- the user must grant Automation permissions for the controlling process

Ghostty AppleScript capabilities relevant to this project include:

- reading windows, tabs, terminals, and terminal working directories
- focusing terminals
- activating windows
- creating windows and tabs with an initial working directory

## Terminology

- `project`: canonical filesystem path used as the session identity
- `window session`: a Ghostty window associated with a project
- `derived project path`: project path inferred from the first tab or terminal
- `state store`: local JSON file with metadata not owned by Ghostty

## First-Pass Scope

### Supported Commands

Planned early commands:

- `ls`: list Ghostty windows and derived project paths
- `switch`: open interactive picker and focus the selected project window
- `open <path>`: focus matching window or create a new window at the path
- `tab [path]`: create a tab in the matching or current project window

The first implementation target is `ls`.

### Window Identity

The working directory of the first tab is the initial heuristic for determining
the project's path for a Ghostty window.

Important caveat:

- shell working directories are mutable
- a user can `cd` away from the original project path

Design response:

- use Ghostty-derived working directory for discovery
- persist a canonical project path in the local state store once a window is
  known
- prefer explicit state over re-deriving identity every time when available

## Architecture

### Components

1. CLI layer
   Parses subcommands and output mode.
2. Ghostty adapter
   Executes AppleScript through `osascript` and parses structured output.
3. Domain model
   Represents windows, tabs, projects, and usage metadata.
4. State store
   Loads and saves JSON metadata.
5. Search and ranking
   Combines fuzzy match score, basename preference, and recency.
6. TUI
   Presents interactive selection for `switch`.

### Suggested Module Layout

Possible initial layout:

- `src/main.rs`
- `src/cli.rs`
- `src/ghostty.rs`
- `src/applescript.rs`
- `src/model.rs`
- `src/state.rs`
- `src/search.rs`
- `src/tui.rs`
- `src/error.rs`

This does not need to exist immediately; it is a target shape as the codebase
grows.

## Data Model

### Runtime Model

```text
GhosttyWindow
- window_id: i64
- window_name: Option<String>
- project_path: Option<PathBuf>
- tabs: Vec<GhosttyTab>

GhosttyTab
- tab_id: i64
- tab_name: Option<String>
- index: usize
- terminals: Vec<GhosttyTerminal>

GhosttyTerminal
- terminal_id: i64
- working_directory: Option<PathBuf>
```

### Persistent State Model

```json
{
  "version": 1,
  "projects": [
    {
      "project_path": "/Users/example/src/project-a",
      "last_selected_at": "2026-04-15T12:00:00Z",
      "selection_count": 42,
      "last_window_id": 1234,
      "aliases": ["project-a"]
    }
  ]
}
```

### Notes On Persistence

- `version` allows lightweight future migrations
- `project_path` is the stable key
- `last_window_id` is a hint, not a trusted permanent identifier
- timestamps should be stored in UTC

## AppleScript Strategy

The Ghostty adapter should prefer one script per logical action.

Examples:

- one script to list all windows and their first-tab working directories
- one script to focus a window or terminal
- one script to create a new window with an initial working directory

AppleScript output should be machine-oriented, not presentation-oriented.

Preferred formats:

- TSV for simple list output
- JSON only if there is a clean and dependable way to emit it

TSV is likely the simplest starting point because it is easy to generate from
AppleScript and easy to parse in Rust.

## Ranking Strategy

The switcher should combine:

- fuzzy score over the full path
- extra weight for basename matches
- recency from the local state file
- optional exact-match bonuses

The final scoring model does not need to be perfect initially. It only needs to
preserve the most important behavior from the tmux flow:

- recently used projects rise to the top
- project-name matches usually beat deep path matches

## CLI Behavior

### `ls`

Responsibilities:

- query Ghostty once
- derive a project path for each window
- display a stable summary

Potential output fields:

- window id
- window name
- project path
- tab count

Possible future modes:

- table output
- JSON output

### `switch`

Responsibilities:

- load Ghostty windows
- load persistent state
- rank entries
- run interactive search
- focus the selected window
- update MRU state

### `open <path>`

Responsibilities:

- normalize input path
- find matching project window if it exists
- otherwise create a new Ghostty window with that working directory
- persist project metadata

## Error Handling

Important failure cases:

- Ghostty not running
- Ghostty installed but AppleScript unavailable
- Automation permission denied
- no readable working directory for a window
- corrupted or missing JSON state file

Design stance:

- fail with plain, actionable errors
- keep state-file recovery simple
- avoid silent fallbacks that obscure why automation failed

## Observability

Useful early diagnostics:

- `--json` output for machine inspection
- `--verbose` for printing AppleScript invocation details
- clear parse errors when script output is malformed

## Open Questions

- Should project identity be strictly path-based, or should user-defined aliases
  be first-class early on?
- Should a Ghostty window be matched by first-tab path only, or should all tabs
  be scanned when looking for a project?
- Should the tool focus a window directly, or focus a specific terminal inside
  that window for more predictable behavior?
- Where should the JSON state file live by default?

## Proposed Default State File Location

Unless repository-local state is explicitly desired, a reasonable default is:

`~/.local/state/ghostty-session-manager/state.json`

This keeps usage history separate from the repository while remaining easy to
find and inspect.

## Implementation Plan

1. Implement `ls` using a single AppleScript query and simple stdout output.
2. Add Rust domain types and parsing for Ghostty query results.
3. Add JSON state loading and saving.
4. Build the interactive `switch` TUI with `ratatui` and `frizbee`.
5. Add `open` and `tab` commands.
6. Refine heuristics around project identity and window reuse.

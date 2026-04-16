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

### Project Identity

Project identity is strictly path-based in the first version.

Design consequence:

- `project_path` is the primary key in persisted state
- aliases are deferred until a real need appears
- matching behavior should stay simple and predictable early on

### Source Of Truth

Ghostty is the source of truth for live runtime state.

The local JSON state file is supplemental metadata only.

Design consequence:

- windows, tabs, terminals, and focus are always discovered from Ghostty
- local state stores only durable metadata such as MRU history
- the application should not attempt to maintain an authoritative mirror of the
  full Ghostty runtime graph

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
- only inspect the first tab in the first version

Scanning all tabs is deferred. If added later, that would mean using any tab's
working directory as a lookup signal for finding the right window. It would not
mean switching directly to a specific tab in the first pass unless that becomes
an explicit product decision.

### Focus Behavior

The first version should focus a Ghostty window directly.

More granular terminal or tab focus can be added later if direct window focus
proves too imprecise in practice.

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
      "last_window_id": 1234
    }
  ]
}
```

### Notes On Persistence

- `version` allows lightweight future migrations
- `project_path` is the stable key
- `last_window_id` is a hint, not a trusted permanent identifier
- timestamps should be stored in UTC
- persisted state should avoid storing full live window or tab inventories as
  authoritative data

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

## Sync Strategy

The first version should use on-demand refresh, not continuous synchronization.

Ghostty does not currently provide a clean event subscription model for window,
tab, terminal, or focus changes through the chosen integration path, so the
application should avoid trying to track live changes incrementally in the
background.

### Command Boundary Refresh

At the start of each command such as `ls`, `switch`, `open`, or `tab`:

1. query Ghostty for the current live snapshot
2. load local JSON metadata
3. merge live state with local metadata in memory
4. perform the requested action
5. persist any metadata updates after a successful action

This keeps the model simple and correct even if Ghostty was changed outside the
tool between invocations.

### Interactive Switcher Refresh

For the `switch` TUI:

- fetch one Ghostty snapshot when the UI starts
- search and rank locally in memory while the UI is open
- optionally support manual refresh later
- avoid background polling in the first version

### Optimistic Updates

After app-initiated actions such as focusing a window or creating a new one,
the application may update in-memory state optimistically for the remainder of
the current command. Persisted metadata should still be limited to supplemental
state, not treated as a complete live runtime snapshot.

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

## Default State File Location

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

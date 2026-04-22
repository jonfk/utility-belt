# Ghostty Session Manager Design

## Overview

`ghostty-session-manager` is a Rust CLI for treating Ghostty windows as
project-scoped sessions. It uses Ghostty's AppleScript API as the control plane
for reading window state and performing actions such as focusing windows.

The implementation should be based on Ghostty's actual scripting dictionary,
not an assumed AppleScript surface. A checked-in snapshot of the dictionary
used for development lives at `reference/Ghostty.sdef`.

Current baseline:

- Ghostty version: `1.3.1`
- bundle build: `15212`
- snapshot date: `2026-04-15`

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
- enough for MRU tracking, fast switch startup, and lightweight metadata

SQLite can be introduced later if the state model becomes more complex.

### Project Identity

Project identity is strictly path-based in the first version.

Design consequence:

- `project_path` is the primary key in persisted state
- live Ghostty windows are still keyed by `window_id`, not by `project_path`
- multiple live windows may share the same canonical `project_path`
- aliases are deferred until a real need appears
- matching behavior should stay simple and predictable early on

This means the persistence model is project-scoped, while the live runtime
model is window-scoped. A single project record may point at a preferred window
for that project, but it does not imply that only one live Ghostty window can
exist for that path.

### Source Of Truth

Ghostty is the source of truth for live runtime state and imperative actions.

The local JSON state file is also used as a cached switch index.

Design consequence:

- `ls` and live reconciliation steps query Ghostty directly
- `switch` may render immediately from cached project records before a live
  Ghostty query completes
- local state may store denormalized hints such as last seen window id and last
  seen window name
- the application should not attempt to maintain an authoritative mirror of the
  full Ghostty runtime graph

### Interactive UI

Interactive switching will use:

- `ratatui` for the terminal UI
- `frizbee` for fuzzy matching and ranking support

This avoids shelling out to `fzf` and keeps control of scoring and display
inside the application.

The `switch` flow should be built in phases:

- phase 2: basic browse-and-select TUI shell
- phase 3: standalone search and ranking logic in Rust
- phase 4: stale-first filtering and ranking inside the TUI with live
  reconciliation

## Platform Constraints

- macOS only
- Ghostty AppleScript support must be enabled
- the user must grant Automation permissions for the controlling process

Ghostty AppleScript capabilities relevant to this project include:

- reading windows, tabs, terminals, and terminal working directories
- reading `selected tab`, `focused terminal`, and tab `index`
- activating windows

The current dictionary is shipped inside the app bundle at:

`/Applications/Ghostty.app/Contents/Resources/Ghostty.sdef`

The current checked-in snapshot was taken from:

- Ghostty version `1.3.1`
- bundle build `15212`
- app bundle path `/Applications/Ghostty.app`

Important observations from the real dictionary:

- `window.id`, `tab.id`, and `terminal.id` are `text`, not integers
- `tab.index` is the only integer-like stable ordering field exposed for tabs
- working directory is exposed on `terminal`, not `window` or `tab`
- the action verbs are concrete AppleScript commands such as `activate window`
  and `focus`

## Terminology

- `project`: canonical filesystem path used as the session identity
- `window session`: a Ghostty window associated with a project path
- `derived project path`: project path inferred from the first terminal in the
  first tab
- `state store`: local JSON file with metadata not owned by Ghostty
- `switch index`: cached per-project metadata used to render the `switch`
  picker before live refresh completes

Cardinality note:

- one project path may map to zero, one, or many live Ghostty windows
- one live Ghostty window maps to at most one derived project path in the first
  version

## First-Pass Scope

### Supported Commands

Planned early commands:

- `ls`: list Ghostty windows and derived project paths
- `switch`: open interactive picker and focus the selected project window

The first implementation target is `ls`.

### Window Identity

The working directory of the first terminal in the first tab is the initial
heuristic for determining the project's path for a Ghostty window.

Important caveat:

- shell working directories are mutable
- a user can `cd` away from the original project path

Design response:

- use Ghostty-derived working directory for live discovery and reconciliation
- persist a canonical project path and last-seen window hints in the local
  state store once a window is known
- allow `switch` to search persisted project records without waiting for a
  fresh Ghostty query
- prefer explicit state over re-deriving identity every time when available
- only inspect the first terminal in the first tab in the first version

Scanning all tabs is deferred. If added later, that would mean using any tab's
working directory as a lookup signal for finding the right window. It would not
mean switching directly to a specific tab in the first pass unless that becomes
an explicit product decision.

### Focus Behavior

The first version should focus a Ghostty window directly.

More granular terminal or tab focus can be added later if direct window focus
proves too imprecise in practice.

## Architecture

### Architectural Style

The implementation should follow a light ports-and-adapters approach.

This is not intended to be a full clean-architecture design with traits and
indirection at every layer. The goal is to keep the code modular and easy to
change without turning a small CLI into a framework exercise.

Design consequences:

- keep the domain and application logic separate from AppleScript execution
- isolate Ghostty-specific integration details in infrastructure code
- avoid introducing traits until there is a concrete need for substitution or
  test doubles
- prefer straightforward concrete types and modules over abstract interfaces in
  the first version

### Components

1. CLI layer
   Parses subcommands and output mode.
2. Application layer
   Orchestrates command workflows such as listing windows and switching to an
   existing project window.
3. Ghostty integration
   Executes AppleScript through `osascript` and parses structured output.
4. Domain model
   Represents windows, tabs, projects, and usage metadata.
5. State store
   Loads and saves JSON metadata.
6. Search and ranking
   Combines fuzzy match score, basename preference, and recency.
7. TUI
   Presents interactive selection for `switch`.

### Boundary Decisions

The Ghostty integration should start as a concrete implementation rather than a
trait-backed port.

Reasoning:

- only a small set of operations is currently required
- there is only one known backend: Ghostty via AppleScript
- introducing a trait before it is needed would add ceremony without improving
  clarity

The first concrete Ghostty API should be minimal:

- `query_windows`
- `focus_window`

The currently implemented `query_windows` boundary is based on the real
dictionary and returns string IDs for windows, tabs, and terminals.

If the AppleScript boundary becomes harder to test, a second backend appears,
or the application layer starts depending on integration details, that is the
time to extract a formal port trait.

### Suggested Module Layout

Possible initial layout:

- `src/main.rs`
- `src/cli.rs`
- `src/app.rs`
- `src/domain.rs`
- `src/ghostty.rs`
- `src/ghostty/applescript.rs`
- `src/state.rs`
- `src/search.rs`
- `src/tui.rs`
- `src/error.rs`

This does not need to exist immediately; it is a target shape as the codebase
grows.

The short runtime type names in this document assume they live in a clearly
named Ghostty-focused module such as `ghostty` or `ghostty::model`.

## Data Model

### Runtime Model

```text
WindowInventory
- windows: Vec<Window>

Window
- window_id: String
- window_name: Option<String>
- project_path: Option<PathBuf>
- tabs: Vec<Tab>

Tab
- tab_id: String
- tab_name: Option<String>
- index: usize
- terminals: Vec<Terminal>

Terminal
- terminal_id: String
- working_directory: Option<PathBuf>
```

### Persistent State Model

```json
{
  "version": 2,
  "projects": {
    "/Users/example/src/project-a": {
      "last_accessed_at": "2026-04-15T12:00:00Z",
      "last_seen_at": "2026-04-15T12:05:10Z",
      "last_window_id": "tab-group-600002952eb0",
      "last_window_name": "project-a"
    }
  }
}
```

### Notes On Persistence

- `version` allows lightweight future migrations
- `project_path` is the stable key and should be the map key in persisted state
- the state file doubles as a cached project index for `switch`
- `last_window_id` is a hint for the preferred window for a project, not a
  guarantee that the project has only one live window
- Ghostty currently exposes window IDs as stable text values, so persisted IDs
  should be strings
- timestamps should be stored in UTC
- `last_seen_at` is a freshness hint for cached switch rows
- `last_window_name` is optional display metadata for cached rows
- persisted state should avoid storing full live window or tab inventories as
  authoritative data
- stale project records are acceptable as cache entries as long as selection
  resolution can recover or fail clearly

## AppleScript Strategy

The Ghostty integration layer should prefer one script per logical action.

Examples:

- one script for `query_windows`
- one script to focus a window

AppleScript output should be machine-oriented, not presentation-oriented.

Preferred formats:

- TSV for simple list output
- JSON only if there is a clean and dependable way to emit it

TSV is likely the simplest starting point because it is easy to generate from
AppleScript and easy to parse in Rust.

For the implemented `ls` command, the TSV row shape is:

- `window_id`
- `window_name`
- `tab_id`
- `tab_index`
- `tab_name`
- `terminal_id`
- `working_directory`

This mirrors the fields exposed by the current Ghostty dictionary closely and
keeps the AppleScript script dumb while Rust owns grouping and derivation.

## Sync Strategy

The first version should use command-scoped refresh, not continuous
synchronization.

Ghostty does not currently provide a clean event subscription model for window,
tab, terminal, or focus changes through the chosen integration path, so the
application should avoid a daemon or continuous polling in the background.
Instead, each command should choose its own freshness strategy.

### `ls` Refresh Strategy

`ls` should favor correctness over startup latency.

At the start of `ls`:

1. query Ghostty for the current live snapshot
2. load local JSON metadata
3. merge live state with local metadata in memory
4. render the current inventory

This keeps listing accurate even if Ghostty was changed outside the tool
between invocations.

### `switch` Refresh Strategy

`switch` should favor startup latency over perfect first-paint accuracy.

At the start of `switch`:

1. load the local JSON state file
2. build picker rows from the cached switch index
3. render the picker immediately from cached data
4. start at most one background `query_windows` call while the UI is open
5. merge live data into the in-memory picker state when the query completes
6. persist refreshed hints after a successful live merge or successful
   selection

Design constraints:

- background refresh must not block first paint or first keystroke
- there should be no periodic polling in the first version
- manual refresh can be added later if one-shot background reconciliation is
  insufficient
- live merges should preserve the current query and selection state
- when possible, rows should be updated in place rather than fully rebuilt to
  reduce visible layout shift
- cached rows remain project-scoped even though live refresh may discover
  multiple windows for the same project path

If live reconciliation finds duplicate windows for a single project path, the
application should not assume that the project row uniquely identifies one live
window. The cached row can continue to represent the project, but the live
state must keep the matching `window_id` values distinct.

### Selection Resolution

When the user confirms a `switch` selection:

1. if the selected project record has a `last_window_id`, attempt to focus that
   window directly
2. if focusing by cached window id fails, query Ghostty once and resolve by
   canonical project path
3. if exactly one matching live window is found, focus it and update cached
   hints
4. if multiple matching live windows are found, prefer the cached
   `last_window_id` when it is still live; otherwise surface the live windows
   as separate choices instead of guessing
5. if no matching live window is found, fail with a clear error or hand off to
   a future create/open flow

This keeps the common path fast while still recovering from stale cache data.

### Optimistic Updates

After app-initiated actions such as focusing a window, the application may
update the in-memory switch index optimistically for the remainder of the
current command. Persisted metadata should still be limited to supplemental
state and cached hints, not treated as a complete live runtime snapshot.

## Ranking Strategy

The switcher should combine:

- fuzzy score over the full path
- extra weight for basename matches
- recency from the local state file
- optional exact-match bonuses
- optional preference for entries confirmed by the most recent live refresh

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

- load persistent state
- render an interactive picker immediately from cached project records
- refresh live Ghostty windows once in the background
- focus the selected window using cached hints first and live reconciliation
  second
- handle duplicate live windows for the same project path without collapsing
  them into one runtime identity
- update MRU state and cached window hints

Staged delivery:

- initial TUI shell supports browse, selection movement, confirm, and cancel
- cached switch startup can ship before background live merge exists
- search and ranking are implemented separately, then integrated into the TUI
- background live merge and selection fallback can follow once the cached path
  is stable

## Error Handling

Important failure cases:

- Ghostty not running
- Ghostty installed but AppleScript unavailable
- Automation permission denied
- Ghostty AppleScript dictionary changed since the snapshot the tool was built
  against
- cached `last_window_id` no longer points at a live window
- no readable working directory for a window
- corrupted or missing JSON state file

Design stance:

- fail with plain, actionable errors
- keep state-file recovery simple
- recover from stale cache with one live reconciliation query when feasible
- avoid silent fallbacks that obscure why automation failed

## Observability

Useful early diagnostics:

- `--json` output for machine inspection
- `--verbose` for printing AppleScript invocation details
- clear parse errors when script output is malformed
- timing breakdown between cached picker startup, live refresh, and selection
  reconciliation
- a checked-in dictionary snapshot in `reference/Ghostty.sdef` for diffing
  against future Ghostty releases

## Default State File Location

Unless repository-local state is explicitly desired, a reasonable default is:

`~/.local/state/ghostty-session-manager/state.json`

This keeps usage history separate from the repository while remaining easy to
find and inspect.

## Implementation Plan

1. Implement `ls` using a single AppleScript query and simple stdout output.
2. Add Rust domain types and parsing for Ghostty query results.
3. Add JSON state loading and saving.
4. Extend persisted project records with cached switch hints such as
   `last_window_id` and `last_seen_at`.
5. Build a basic interactive `switch` TUI shell with `ratatui` that can render
   from cached state immediately.
6. Add standalone search and ranking logic with `frizbee`.
7. Plug search into the `switch` TUI.
8. Add one-shot live reconciliation for `switch`, including in-place UI merge
   and selection fallback by project path.
9. Refine heuristics around project identity, stale-record pruning, and window
   reuse.

# Ghostty Session Manager Implementation Plan

This document is intentionally about implementation status and next steps, not
the full target architecture. The design doc can describe the desired end
state more broadly. This plan should stay aligned with what already exists in
the codebase so the next implementor can see what is done and what still
remains.

## Current Status

The following pieces are already implemented:

- Rust crate scaffolded with `ls` and `switch` commands.
- Live Ghostty AppleScript integration for:
  - querying windows
  - focusing a window by id
- TSV parsing and runtime grouping into `WindowInventory`.
- Table and JSON output for `ls`.
- JSON state storage keyed by canonical project path.
- Persisted MRU metadata with `last_accessed_at`.
- Cached switch hints in persisted project state:
  - `last_window_id`
  - `last_seen_at`
  - optional `last_window_name`
- Joining live inventory with persisted state for display and switching.
- Refreshing cached project records from live Ghostty inventory during `ls`.
- Cached-first `switch` startup from persisted project state.
- One-time live seeding when `switch` runs with an empty state file.
- Cached picker rows built from persisted project records.
- MRU ordering for cached switch rows using `last_accessed_at`.
- Deterministic tie-breaking for equal MRU timestamps by canonical project key.
- Dedicated `search.rs` module for cached project ranking.
- `frizbee`-backed fuzzy ranking over persisted project records.
- Case-insensitive query normalization for cached ranking.
- Ranking heuristics for cached project records:
  - exact basename match
  - exact full-path match
  - basename prefix match
  - basename-hit preference over full-path hits when the basename score is stronger
  - fuzzy score
  - MRU weighting
  - canonical project key tie-breaking
- Query-driven `ratatui` switcher with:
  - typed filtering over cached project rows
  - selection movement within filtered results
  - selection preservation by canonical project key when still visible
  - fallback to the first filtered result when the previous selection drops out
  - empty-state rendering when no cached projects match
  - confirm
  - cancel
- One-shot background live reconciliation while the picker is open.
- Query/selection-preserving live refresh merges in the picker.
- Stale cached window-id fallback to live project-path resolution during
  switch completion.
- Updating MRU state after a successful switch.
- Updating cached window hints after a successful switch.
- Debug timing output via `--debug`.

The following pieces are not implemented yet:

- Explicit stale-record pruning or richer project identity reconciliation.

## Current Behavior

Today the command behavior is:

- `ls`
  - queries Ghostty live
  - loads persisted state
  - refreshes cached project records from live windows
  - persists state if the refreshed cache changed
  - merges live and persisted data in memory
  - renders either table or JSON output
- `switch`
  - loads persisted state
  - if cached projects exist, opens a query-driven picker from cached project rows immediately
  - if the state file is empty, performs a one-time live Ghostty query to seed state
  - starts with cached rows ordered by MRU with deterministic tie-breaking
  - updates filtered results as the user types, using cached ranking from `search.rs`
  - starts at most one background live Ghostty refresh while the picker is open
  - merges refreshed cached rows into the picker without losing the current query or selection
  - preserves the current selection when the selected project remains in the filtered set
  - falls back to the first filtered result when the previous selection disappears
  - shows an empty state when the current query has no cached matches
  - attempts to focus the selected cached window id first
  - if the cached window id is stale, falls back to live resolution by canonical project path
  - when multiple live windows match a stale cached project, chooses the first live match in inventory order
  - records `last_accessed_at` and cached window hints for the selected project

That means `switch` now uses a stale-first cached path. Live reconciliation
and stale-id recovery are now implemented, while identity/canonicalization edge
cases and stale-record cleanup are still deferred to later phases.

## Completed Work

### Phase 1: Extend State For Cached Switching

Phase 1 is now implemented.

- Project records now persist:
  - `last_accessed_at`
  - `last_window_id`
  - `last_seen_at`
  - optional cached display metadata like `last_window_name`
- Keep the state file keyed by canonical project path.
- The state file still uses `version: 1`.
- There is no migration path for older partial project records. Non-conforming
  state files fail load with an error.
- Continue treating persisted window ids as hints, not authoritative truth.
- When multiple live windows share the same canonical project path, cache
  refresh keeps the existing preferred `last_window_id` if it is still live.
  Otherwise it falls back to the first live match in inventory order.

Implementation note:

- New project records discovered during live refresh are currently bootstrapped
  with `last_accessed_at = observed_at` because the persisted schema requires
  `last_accessed_at` to be present.

Verification already in code:

- Unit tests cover saving and loading the extended project records.
- Unit tests cover failure on old incomplete project record shapes.
- Unit tests cover duplicate-window preference retention and stale-preference
  fallback.
- `ls --json` exposes the richer persisted state fields.

### Phase 2: Make `switch` Cached-First

Phase 2 is now implemented.

- `prepare_switch` now loads persisted state before querying Ghostty.
- When cached projects already exist, `prepare_switch` builds `SwitchContext`
  directly from persisted project records.
- When the state file is empty, `prepare_switch` performs a one-time live seed
  via `query_windows` plus `refresh_from_inventory`.
- Cached switch rows use:
  - persisted `last_window_id`
  - optional persisted `last_window_name`
  - the persisted canonical project key as the project path
- Cached row titles use the final path segment when available, with full-path
  fallback.
- Cached row details omit the window-name segment when it is absent.
- Cached rows are ordered by:
  - `last_accessed_at` descending
  - canonical project key ascending for deterministic tie-breaking
- `complete_switch` behavior is unchanged and still treats cached window ids as
  hints rather than authoritative truth.

Verification already in code:

- Application tests cover building switch rows from cached state without live
  Ghostty input.
- Application tests cover MRU ordering and deterministic tie-breaking.
- Application tests cover cached title/detail rendering.
- Application tests cover populated-state startup without a live query.
- Application tests cover empty-state live seeding.
- Application tests cover the empty-live-seed no-windows error path.
- Manual Ghostty smoke tests have not yet been recorded in this plan.

### Phase 3: Add Search And Ranking

Phase 3 is now implemented.

- Added a dedicated `search.rs` module.
- Added `frizbee` as the fuzzy-matching backend.
- Search ranks cached project records rather than requiring live window rows.
- Query normalization now:
  - trims surrounding whitespace
  - lowercases query and candidate strings
  - treats an empty query as "show all"
- Ranking now combines:
  - exact basename match
  - exact full-path match
  - basename prefix match
  - basename-hit preference when the basename fuzzy score beats the full-path score
  - best fuzzy score
  - `last_accessed_at` descending
  - canonical project key ascending for deterministic tie-breaking
- Empty queries return all project keys in the same MRU order used by cached
  switch rows.
- This phase is intentionally not wired into the TUI yet. The `switch`
  command remains browse-only until phase 4.

Verification:

- Search tests cover:
  - exact basename matches beating deeper full-path fuzzy matches
  - basename prefix matches beating weaker full-path matches
  - partial path queries
  - MRU boosts
  - canonical project key tie-breaking
  - empty-query ordering
  - no-match queries
- A realistic mixed-project fixture covers exact, prefix, and path-oriented
  queries across similar project names.
- `cargo test` passes with the new search module.

### Phase 4: Plug Search Into The TUI

Phase 4 is now implemented.

- `PickerEntry` now carries the canonical project key used by the persisted
  state map.
- The picker state now owns:
  - the full cached entry set keyed by canonical project key
  - the current query string
  - the current filtered project-key order
  - the currently selected project key
- The TUI now calls `search::rank_project_keys` on every query change instead
  of duplicating ranking logic in the UI layer.
- Query editing now supports:
  - printable character input
  - backspace deletion
  - query-aware filtered movement
  - confirm on the current filtered selection
  - cancel via `Esc` or `q`
- Selection behavior now:
  - preserves the selected project when it remains visible after filtering
  - falls back to the first filtered result when it does not
  - clears selection and returns `Cancel` when there are no filtered rows
- The rendered picker now shows:
  - an explicit query field
  - filtered cached rows
  - an empty-state message when no cached projects match
- `cli::run_switch` now passes `context.state.projects` into the picker so the
  UI ranks against the persisted cache directly.
- `complete_switch` behavior is unchanged and still updates MRU state after a
  successful selection.

Verification:

- TUI tests now cover:
  - empty-query MRU ordering
  - query updates and filtered ordering
  - backspace widening the filtered set
  - movement within filtered results
  - selection preservation by project key
  - fallback to the first filtered result
  - empty-query and no-result confirm behavior
  - cancel behavior
- `cargo test` passes with the phase 4 picker changes.

### Phase 5: Add One-Shot Live Reconciliation

Phase 5 is now implemented.

- `SwitchContext` now tracks whether the current switch session was already
  seeded from live Ghostty data so the picker can skip a redundant background
  refresh.
- `switch` now starts at most one background `query_windows` refresh while the
  picker is open when startup did not already seed from live data.
- The picker loop now polls both terminal input and the one-shot refresh
  channel, keeping the UI responsive while waiting for live data.
- Live reconciliation now:
  - refreshes persisted cached hints from live inventory
  - rebuilds cached project rows from updated state
  - reapplies the current query
  - preserves the selected project key when still present
  - otherwise falls back to the first filtered result or clears selection
- Cancel now persists state once when a background live refresh changed the
  cached switch index.
- `complete_switch` now supports stale-id fallback:
  - tries the cached `last_window_id` first
  - uses the prefetched live inventory when available, otherwise performs one
    synchronous `query_windows`
  - resolves live matches by canonical project path
  - focuses the first matching live window in inventory order when the cached
    id is stale
  - refreshes cached hints and records project access after a recovered switch
  - returns a clear error when no live project match exists
- Picker rows remain project-scoped in this phase. Duplicate live windows are
  not surfaced as separate rows in the UI.

Verification:

- TUI tests now cover:
  - applying a live refresh without losing the active query
  - preserving or falling back selection correctly after a refresh
  - introducing newly discovered cached projects without breaking filtered ordering
  - keeping empty-state behavior correct after refresh
- Application tests now cover:
  - reconciling live inventory back into cached switch rows
  - updating `last_seen_at` without changing `last_accessed_at`
  - stale-id fallback using prefetched live inventory
  - stale-id fallback performing one synchronous live query when needed
  - unique and duplicate live-match resolution
  - clear failure when no live project match exists
- `cargo test` passes with the phase 5 reconciliation and fallback changes.

## Remaining Work

### Phase 6: Improve Identity And Cache Hygiene

Tighten the behavior once real usage exposes edge cases.

- Reconcile live Ghostty-derived paths with persisted project identity.
- Decide when persisted identity should win over a drifted shell working
  directory.
- Decide when stale cached project records should be retained, refreshed, or
  pruned.
- Keep the rules simple and inspectable.

Verification:

- Unit tests cover identity reconciliation and stale-record handling.
- Manual smoke test: after changing directories inside a known window,
  switching still behaves predictably.

### Phase 7: Polish Errors, Diagnostics, And Docs

Improve trust and debuggability once the main behavior is in place.

- Improve user-facing errors for stale cache misses, Ghostty failures, and
  Automation permission issues.
- Expand diagnostics where they explain why switching was slow or failed.
- Report timing separately for:
  - cached picker startup
  - live reconciliation
  - final selection resolution
- Update docs to match actual behavior and limitations.

Verification:

- CLI tests or snapshot-style assertions cover key error messages.
- Manual smoke tests cover at least one success path and one failure path for
  each supported command.

## Suggested Delivery Order

The remaining work can be shipped in this order:

1. Phase 6
2. Phase 7

That order keeps the fast-path switch work ahead of the more subtle live
reconciliation and cache-hygiene work.

## Out Of Scope For This Plan

These are reasonable future additions, but they do not need to block the next
usable version:

- scanning every tab as a lookup signal
- direct tab targeting during switch
- pane/layout restoration
- background daemons
- periodic background polling while the picker is open
- SQLite or more complex persistence
- support beyond macOS + Ghostty AppleScript

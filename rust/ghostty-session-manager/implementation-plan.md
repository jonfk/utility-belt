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
- Basic `ratatui` switcher with:
  - browse-only list
  - selection movement
  - confirm
  - cancel
- Updating MRU state after a successful switch.
- Updating cached window hints after a successful switch.
- Debug timing output via `--debug`.

The following pieces are not implemented yet:

- Typed search inside the switcher.
- Wiring cached search/ranking into the picker UI.
- One-shot background live reconciliation while the picker is open.
- Selection fallback from stale cached ids to live resolution by project path.
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
  - if cached projects exist, opens a browse-only picker from cached project rows immediately
  - if the state file is empty, performs a one-time live Ghostty query to seed state
  - orders cached rows by MRU with deterministic tie-breaking
  - focuses the selected window
  - records `last_accessed_at` and cached window hints for the selected project

That means `switch` now uses a stale-first cached path. Live reconciliation
while the picker is open, typed filtering in the picker, and fallback from
stale cached window ids are still deferred to later phases.

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

## Remaining Work

### Phase 4: Plug Search Into The TUI

Add typed filtering to the existing picker.

- Add query input and filtered result updates.
- Preserve selection sensibly as the filtered result set changes.
- Keep the search logic testable outside the terminal UI.
- Continue updating MRU state when a selection is confirmed.

Verification:

- State-oriented tests cover query updates, filtering, movement, confirm, and
  cancel.
- Manual smoke test: typing a query narrows results and selecting still focuses
  the intended window.

### Phase 5: Add One-Shot Live Reconciliation

Recover freshness without moving back to a query-before-render model.

- Start at most one live `query_windows` refresh while the switcher is open.
- Merge live results into the in-memory picker state.
- Update rows in place where practical so the UI does not jump around more than
  necessary.
- Persist refreshed hints after a successful live merge.
- On selection, try cached `last_window_id` first and fall back to live
  resolution by project path when needed.

Verification:

- State-oriented tests cover applying live refresh results without losing the
  active query or selection.
- Unit tests cover fallback from stale cached window id to live project-path
  resolution.
- Manual smoke test: cached rows appear first and reconcile cleanly once live
  data arrives.

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

1. Phase 4
2. Phase 5
3. Phase 6
4. Phase 7

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

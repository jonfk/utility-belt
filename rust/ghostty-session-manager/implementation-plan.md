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
- Joining live inventory with persisted state for display and switching.
- Basic `ratatui` switcher with:
  - browse-only list
  - selection movement
  - confirm
  - cancel
- Updating MRU state after a successful switch.
- Debug timing output via `--debug`.

The following pieces are not implemented yet:

- Cached switch hints such as `last_window_id`, `last_seen_at`, or cached
  display metadata.
- Cached-first `switch` startup. `switch` still blocks on `query_windows`
  before opening the picker.
- Typed search or ranking inside the switcher.
- `frizbee` integration or a dedicated `search.rs` module.
- One-shot background live reconciliation while the picker is open.
- Selection fallback from stale cached ids to live resolution by project path.
- Explicit stale-record pruning or richer project identity reconciliation.

## Current Behavior

Today the command behavior is:

- `ls`
  - queries Ghostty live
  - loads persisted state
  - merges the two in memory
  - renders either table or JSON output
- `switch`
  - queries Ghostty live before showing the UI
  - loads persisted state
  - opens a browse-only picker from the live window list
  - focuses the selected window
  - records `last_accessed_at` for the selected project

That means the current implementation is still on the old query-first switch
path. The next plan should move from that baseline toward the newer stale-first
switch design.

## Remaining Work

### Phase 1: Extend State For Cached Switching

Add the minimum persisted fields needed for a fast switch path.

- Extend project records with cached switch hints such as:
  - `last_window_id`
  - `last_seen_at`
  - optional cached display metadata like `last_window_name`
- Keep the state file keyed by canonical project path.
- Add schema versioning or migration handling as needed so existing state files
  do not break silently.
- Continue treating persisted window ids as hints, not authoritative truth.

Verification:

- Unit tests cover loading old and new state shapes if a migration is needed.
- Unit tests cover saving and loading the extended project records.
- Manual smoke test: switching and listing update the new cached fields in the
  state file.

### Phase 2: Make `switch` Cached-First

Change switch startup so the picker can appear before a live Ghostty query
completes.

- Build picker rows from persisted project state when possible.
- Keep a reasonable bootstrap behavior for first use when the state file is
  empty. The simplest acceptable path is a one-time live seed.
- Preserve the current basic picker behavior: browse, confirm, cancel.
- Keep focus behavior unchanged on the happy path.

Verification:

- State-oriented tests cover building switch rows from cached state without
  live Ghostty input.
- Manual smoke test: with populated state, `switch` opens immediately from
  cached data.
- Manual smoke test: first-use behavior still works when no cached projects
  exist.

### Phase 3: Add Search And Ranking

Implement the pure Rust search path before wiring it into the UI.

- Add a dedicated search/ranking module.
- Rank using cached project records rather than requiring live window rows.
- Start with:
  - fuzzy path matching
  - basename preference
  - MRU weighting
- Define deterministic tie-breaking.

Verification:

- Table-driven tests cover basename matches, partial path matches, MRU boosts,
  and tie-breaking.
- A realistic fixture test covers ranking across a mixed project set.

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

1. Phase 1
2. Phase 2
3. Phase 3
4. Phase 4
5. Phase 5
6. Phase 6
7. Phase 7

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

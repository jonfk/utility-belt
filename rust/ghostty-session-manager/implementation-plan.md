# Ghostty Session Manager Implementation Plan

This plan assumes the current baseline already exists:

- Rust crate scaffolded
- `ls` command implemented
- AppleScript window inventory query working
- parser and table rendering covered by initial unit tests

The goal of this plan is to keep each phase small enough to ship and verify on
its own. Each phase should leave the project in a usable state, even if later
phases are not started yet.

## Phase 0: Harden The Inventory Baseline

Focus on making the existing `ls` path trustworthy before adding more commands.

- Tighten parsing and domain behavior around empty values, tab ordering, and
  malformed AppleScript output.
- Add a few more tests around `WindowInventory` derivation and parser edge
  cases.
- Confirm the CLI output is stable enough for both human-readable and JSON
  usage.

Verification:

- `cargo test` covers empty inventories, missing working directories, multiple
  tabs out of order, and malformed TSV rows.
- Manual smoke test: `ghostty-session-manager ls` works against a live Ghostty
  instance with multiple windows.

## Phase 1: Add Local State Storage

Introduce a small JSON state file for metadata Ghostty does not own.

- Add `state.rs` and define a simple on-disk format keyed by canonical project
  path.
- Store MRU-style metadata such as `last_accessed_at`.
- Keep the state model narrow so it can be inspected and reset by hand.

Verification:

- Unit tests cover state load, save, empty-file behavior, and round-tripping.
- Unit tests cover merging live Ghostty inventory with persisted metadata.
- Manual smoke test: the JSON file is created, readable, and updates after a
  command that touches project state.

## Phase 2: Implement `open <path>`

Add the first stateful workflow: focus an existing project window or create a
new one.

- Canonicalize the requested path before matching.
- Add the minimal AppleScript actions needed for `focus_window` and `new_window`.
- Reuse an existing matching window when possible and only create a new window
  when no match exists.

Verification:

- Unit tests cover path canonicalization and the decision to focus vs create.
- AppleScript command construction is covered by focused tests where practical.
- Manual smoke test: running `open <path>` twice focuses the same window on the
  second run instead of creating a duplicate.

## Phase 3: Build Search And Ranking

Create the pure Rust logic that decides which project should rank highest.

- Add `search.rs` with fuzzy matching, basename preference, and MRU weighting.
- Keep ranking separate from the TUI so it can be tested without terminal
  interaction.
- Decide on a stable ordering rule for ties.

Verification:

- Table-driven tests cover exact basename matches, partial path matches, MRU
  boosts, and deterministic tie-breaking.
- A small fixture-driven test verifies ranking against a realistic set of
  project paths.

## Phase 4: Add Interactive `switch`

Wrap the ranking engine in a terminal picker.

- Add `ratatui`-based selection UI for project switching.
- Show enough context to disambiguate similar project names.
- On selection, focus the chosen window and update MRU state.

Verification:

- Reducer- or state-oriented tests cover selection movement, filtering, and
  cancel behavior.
- Manual smoke test: open the picker, type a query, hit enter, and verify the
  intended Ghostty window is focused.
- Manual smoke test: escape or cancel leaves Ghostty unchanged.

## Phase 5: Add `tab [path]`

Support expanding a project window without creating a new project session.

- Add the AppleScript action for creating a tab with an initial working
  directory.
- Resolve the target window by explicit path first; later fallback behavior can
  stay simple.
- Keep the first version focused on creating tabs, not on complex tab targeting.

Verification:

- Unit tests cover target-window resolution and command argument handling.
- Manual smoke test: `tab /path/to/project` adds a tab in the matching project
  window with the expected initial working directory.

## Phase 6: Improve Project Identity Robustness

Make matching resilient once real usage starts exposing edge cases.

- Reconcile live Ghostty-derived paths with persisted project identity.
- Prefer persisted identity when a known project window has drifted because the
  shell `cd` changed.
- Add clear rules for when live data wins and when persisted metadata wins.

Verification:

- Unit tests cover identity reconciliation and path normalization rules.
- Manual smoke test: after a terminal changes directories away from the project
  root, `open <project>` still finds the expected window once the project has
  been learned.

## Phase 7: Polish Errors, Diagnostics, And Docs

Make the tool easier to trust and debug in daily use.

- Improve errors for Ghostty not running, missing Automation permissions, and
  invalid project paths.
- Expand `--verbose` diagnostics where they help explain AppleScript failures.
- Update docs with setup steps, command behavior, and known limitations.

Verification:

- CLI tests or snapshot-style assertions cover key error messages.
- Manual smoke tests cover at least one failure path and one successful path for
  each supported command.
- A new user can follow the docs to get `ls`, `open`, and `switch` working on a
  fresh machine.

## Suggested Delivery Order

If the project needs especially small PRs, phases can be split this way:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Phase 5
7. Phase 6
8. Phase 7

That order keeps the risky Ghostty automation work early, the search logic pure
and testable, and the TUI late enough that the underlying workflows already
exist.

## Out Of Scope For This Plan

These are still reasonable future additions, but they do not need to block the
first usable version:

- scanning every tab as a lookup signal
- direct tab targeting during switch
- pane/layout restoration
- background daemons
- SQLite or more complex persistence
- support beyond macOS + Ghostty AppleScript

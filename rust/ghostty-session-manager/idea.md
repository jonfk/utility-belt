# Ghostty Session Manager

## Problem

Tmux sessions are currently being used as a project-oriented workspace manager.
Each project has its own session, rooted at the project's working directory, with
multiple windows and panes representing ongoing work. Session switching is fast
because it is driven by search and recency.

Ghostty windows and tabs can provide a similar workflow, but Ghostty does not
natively present project windows as a searchable, session-like abstraction.

## Goal

Create a CLI-first tool that treats Ghostty windows like project sessions.

Each Ghostty window should represent an active project workspace:

- A window maps to one project path, but a project may have multiple Ghostty
  windows in practice.
- Tabs inside that window represent different work contexts for the same project.
- The project identity is based on a working directory path.
- Switching between projects should feel as fast and intentional as switching
  tmux sessions.

## User Experience

The workflow should support:

- Viewing currently open Ghostty project windows.
- Searching project windows by working directory path.
- Preferring matches on the final path segment, since that is usually the
  project name.
- Prioritizing recently used project windows.
- Focusing a preferred existing project window instead of creating unnecessary
  duplicates.
- Creating a new project window when one does not already exist.
- Expanding an existing project window with more tabs over time.

## Mental Model

The tool creates a lightweight session layer on top of Ghostty:

- `tmux session` becomes `Ghostty window`
- `tmux windows/panes` becomes `Ghostty tabs/terminals`
- `session switcher` becomes `project window switcher`

The project path is the session identity. Specific live windows are still
identified separately by Ghostty window ID, because more than one live window
may exist for the same project path.

The point is not to recreate tmux exactly. The point is to preserve the useful
parts of the workflow:

- project-oriented organization
- fast switching
- recency-aware navigation
- low-friction creation and reuse

## Success Criteria

The tool is successful if it becomes a practical replacement for the current
tmux session-switching flow for project navigation inside Ghostty.

That means:

- listing Ghostty windows is reliable
- finding the intended project is fast
- switching is predictable
- state stays simple
- the system is easy to inspect and debug

## Non-Goals For The First Iteration

- Full tmux parity
- Pane layout restoration
- Rich synchronization with arbitrary shell state
- Complex daemon-based background services
- Cross-platform support beyond macOS and Ghostty AppleScript

## Initial Milestones

1. List Ghostty windows and derive a project path for each.
2. Add a project switcher UI with fuzzy search and recency ordering.
3. Persist simple local state for usage history.
4. Focus an existing matching window or create a new one for a project path.
5. Add commands for tab creation and basic project window lifecycle management.

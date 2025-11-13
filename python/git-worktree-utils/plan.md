Proposed Structure

- git_smart_worktree/__init__.py
- cli.py: Typer app registering ls, add, rm, global options.
- config.py: env validation, repo detection (rev-parse, remote get-url), default branch resolution, path builders, slugify helper.
- git.py: thin wrappers for subprocess git calls, porcelain parsing, shared run_git() with admin clone context handling.
- models.py: dataclasses RepoContext, Paths, WorktreeEntry, maybe CommandResult.
- exceptions.py: ConfigError, GitError, ValidationError, UserAbort.
- interactive.py: wrappers around InquirerPy prompts so commands can stay non-interactive-friendly and easy to mock.
- worktrees.py: business functions list_worktrees, add_worktree, remove_worktree, orchestrating config + git + prompts.
- __main__.py or Typer console script entry configured in pyproject.toml.

Each module stays small; no need for further subpackages. Tests under tests/ mirroring modules, using pytest + CliRunner.

Work Plan

- Phase 1: Bootstrap package skeleton, Typer app, exception types, shared subprocess helper, basic env validation (fail fast if vars missing).
- Phase 2: Implement repo discovery, remote parsing, default branch detection, slugify util; add unit tests for parsing/slugging.
- Phase 3: Build worktrees.list_worktrees using admin clone context (stub admin bootstrap), format table/JSON; wire to ls command.
- Phase 4: Implement admin clone bootstrap + fetch logic, worktree path creation, branch existence checks, add_worktree non-interactive flow; support prompts later.
- Phase 5: Add interactive flows (prompts for add/rm) via interactive.py, ensure optional when args provided.
- Phase 6: Implement rm command (path + interactive select), cleanup directories, force flag handling.
- Phase 7: Polish user messaging, loading indicators for long ops, finalize error mapping, enrich CLI help text.
- Phase 8: Testing sweep (unit + CLI), lint/format, update README with usage examples, verify pyproject entry point.

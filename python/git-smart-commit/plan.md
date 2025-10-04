# 1) Project shape

```
git-smart-commit/
├─ pyproject.toml            # managed by uv; console_script entry point
├─ src/
│  └─ git_smart_commit/
│     ├─ __init__.py
│     ├─ cli.py              # Typer app, argument parsing, command wiring
│     ├─ app.py              # top-level orchestration (business flow)
│     ├─ gitio.py            # all git subprocess calls (tiny wrapper layer)
│     ├─ llm/
│     │  ├─ __init__.py
│     │  ├─ protocol.py      # LLMClient Protocol (generate() interface)
│     │  ├─ llm_cli.py       # current adapter: shells out to `llm`
│     │  └─ litellm.py       # (future) LiteLLM adapter, drop-in
│     ├─ models.py           # Typed dicts/dataclasses for staged/unstaged schemas
│     ├─ prompt.py           # Prompt assembly (base + project file + extra text)
│     ├─ render.py           # Rich UI helpers (panels, rules, preview)
│     └─ config.py           # defaults, env, per-repo overrides
└─ tests/
   ├─ test_cli.py
   ├─ test_gitio.py
   └─ fixtures/              # tiny throwaway repos for e2e tests
```

# 2) Dependencies & bootstrapping

* Use **uv**:

  * `uv init --package git-smart-commit`
  * `uv add typer rich`
  * (Optional later) `uv add litellm`
* `pyproject.toml`:

  * `dependencies = ["typer>=0.12", "rich>=13"]`
  * `[project.scripts] git-smart-commit = "git_smart_commit.cli:app"`
* Run via `uvx git-smart-commit ...` or `uv run git-smart-commit ...` during dev.

# 3) CLI surface (Typer)

Single command, compatible flags with your zsh version:

* `-m/--model TEXT` (default `gemini-2.5-flash`)
* `--edit/--no-edit` (default `--edit`: open git editor with a template for multiline)
* `--dry-run` (show what would happen; don’t stage/commit)
* `--verbose` (extra logs)
* `PROMPT... -- [LLM FLAGS…]`

  * Before `--`: additional prompt text (freeform)
  * After  `--`: pass-through flags to the `llm` CLI

Implementation: make the Typer command **allow extra args** and **ignore unknown options**, capturing everything after `--` as `llm_passthrough`.

# 4) Orchestration flow (app.py)

Pseudocode (very high level):

```
def run(model, extra_prompt, llm_flags, edit, dry_run):
    guard.check_requirements(["git", "llm"])
    repo = gitio.ensure_repo()

    context = Context(
        recent_commits = gitio.log_oneline(n=5),
        project_guidelines = config.read_repo_prompt(".git-commit-ai-prompt.txt"),
    )

    base_prompt = prompt.build_base(context, extra_prompt)

    if gitio.has_staged_changes():
        diff = gitio.diff_staged()
        schema = models.staged_schema()
        data = llm_client.generate(diff=diff, schema=schema, prompt=base_prompt,
                                   model=model, extra_flags=llm_flags)
        message, body = models.parse_staged(data)
        plan = Plan(steps=[], commit=Commit(message, body))
    elif gitio.has_unstaged_changes():
        diff = gitio.diff_unstaged()
        schema = models.unstaged_schema()
        data = llm_client.generate(diff=diff, schema=schema, prompt=base_prompt,
                                   model=model, extra_flags=llm_flags)
        files, message, body = models.parse_unstaged(data)
        plan = Plan(steps=[Stage(files)], commit=Commit(message, body))
    else:
        render.info("No changes detected"); return

    render.preview(plan)  # Rich panel: files to stage, commit header/body

    if dry_run: return

    for step in plan.steps: step.apply()  # e.g., git add verified files only

    if body:  # multiline
        temp = gitio.write_temp_commit(message, body)
        gitio.commit_with_editor(template=temp) if edit else gitio.commit_with_file(temp)
    else:
        gitio.commit_single_line(message, edit=edit)  # if edit, open editor w/ message
```

Notes:

* **No `jq`**: parse JSON in Python and validate with our typed structures.
* **Safety**: verify suggested files exist in `git diff --name-only` before staging; warn on mismatches.

# 5) LLM abstraction (for a painless LiteLLM swap later)

`llm/protocol.py`:

```python
class LLMClient(Protocol):
    def generate(self, *, diff: str, schema: dict, prompt: str, model: str, extra_flags: list[str]) -> dict: ...
```

* **Current adapter**: `llm_cli.py`

  * Builds `echo "$prompt\n\nDiff:\n$diff" | llm -m model --schema <json> [extra_flags]`
  * Captures stdout, loads JSON, returns dict
  * Handles non-zero exits and empty outputs with nice Rich errors

* **Future**: `litellm.py`

  * Implements the same `generate(...)` but calls LiteLLM’s Python SDK (e.g., `completion_json_mode` or function-calling equivalent).
  * Because `app.py` only knows about `LLMClient`, the swap = 1-line change in `cli.py` wiring or a config flag `LLM_BACKEND=litellm`.

# 6) Git interactions (gitio.py)

Small, pure functions wrapping `subprocess.run`:

* `ensure_repo()`, `has_staged_changes()`, `has_unstaged_changes()`
* `log_oneline(n)`, `diff_staged()`, `diff_unstaged()`
* `stage(files: list[str])` (only files present in `git diff --name-only`)
* `commit_with_editor(template_path)` → `git commit --edit --template=...`
* `commit_with_file(file)` → `git commit -F file`
* `commit_single_line(message, edit=False)`:

  * if `edit=True`: create temp file with header only and `--edit --template=...`
  * else: `git commit -m message`

# 7) Prompt handling (prompt.py)

* Build the **base prompt** verbatim from your script’s guidance (imperative mood, conventional commits, scope rules, etc.).
* Prepend “Recent commits” and “Project-specific commit guidelines” sections only if present.
* Append `Additional context:` when user provided extra prompt text.

# 8) Schemas & parsing (models.py)

* Keep your two JSON schemas (staged/unstaged) as Python dicts.
* Define `TypedDict`/`dataclass` to represent responses:

  * `StagedResp { message: str; body?: list[str] }`
  * `UnstagedResp { files: list[str]; message: str; body?: list[str] }`
* Add validators:

  * Trim whitespace; drop empty body lines
  * Enforce message length (warn if >100 chars; suggest tightening)

# 9) UI/UX polish (render.py with Rich)

* **Status** spinners for “Analyzing staged diff…” / “Selecting files…”
* **Panel** to preview:

  * **Commit header** (bold)
  * Optional **body** (as a Markdown block or bullet list)
  * If unstaged, a **table** listing files the model suggests staging (with existence/changed status checked and marked ✓/✗)
* Subtle warnings: long subject, missing imperative verb, etc.
* Respect `--verbose` to echo raw JSON from the model if the user wants it.

# 10) Config (config.py)

* Resolution order for defaults:

  1. CLI flags
  2. Env vars: `GSC_MODEL`, `GSC_LLM_FLAGS`, `GSC_BACKEND=llm|litellm`
  3. Per-repo file: `.git-commit-ai-prompt.txt` (content only)
  4. Optional global `~/.config/git-smart-commit/config.toml` (model, backend, default extra flags)
* Keep it minimal in v1; just model + optional default flags.

# 11) Testing approach

* Unit tests for:

  * staged/unstaged parsers (happy & error paths)
  * `gitio` wrappers using a temp repo fixture
  * prompt assembly (recent commits/guidelines/extra prompt)
* E2E smoke in a temp repo:

  * Create a file, make changes, simulate model output via a fake `LLMClient` that returns deterministic JSON.
  * Assert: files staged correctly; commit created; message/body match.

# 12) Migration path to LiteLLM

* Ship v0 with `LlmCliClient` as default.
* Add `LiteLlmClient` behind a flag (no behavior change).
* Once stable, flip default → `LiteLlmClient`, keep CLI fallback with `--backend=llm`.
* (Optional) Add model aliases mapping (e.g., your current `gemini-2.5-flash`) to LiteLLM provider syntax.

# 13) Small UX choices vs your zsh version

* You used `print -z` to push the command into the zsh buffer. In Python, we’ll:

  * Default to `--edit` so users can tweak in their git editor (mirrors your temp-file template behavior).
  * Provide `--dry-run` (show plan, don’t touch the repo) and `--no-edit` (apply directly).
* No external `jq` dependency; JSON is parsed in-process.


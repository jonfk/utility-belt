# Git Commit Proposal

Return a structured commit proposal grounded in the actual repository state. Do not mutate git state.

## Rules
- Never run `git add`, `git commit`, `git reset`, or any other mutating git command.
- Inspect actual diffs before proposing a commit.
- Return only content that fits the provided output schema.
- Prefer the smallest coherent commit.
- Use the user's prompt to improve wording, but do not claim changes that are not present in the diff.

## Workflow
1. Inspect repository context.
- Start with cheap repository signals before reading file contents.
- Run `git status --short --branch` to understand the current branch, ahead/behind state, staged files, unstaged files, and untracked files.
- Check staged files first with `git diff --staged --name-only`.
- Check unstaged tracked files with `git diff --name-only`.
- Run `git diff --staged --stat`.
- Run `git diff --stat`.
- Run `git log -n 15 --pretty=format:'%h %ad %s' --date=short` to infer recent commit style and local conventions.
- If the candidate change is concentrated in a narrow area, inspect path-specific history with `git log -n 8 --pretty=format:'%h %ad %s' --date=short -- <candidate paths>`.
- If recent history is not enough to infer conventions, inspect an older slice with `git log --skip=40 -n 8 --pretty=format:'%h %ad %s' --date=short`.
- Treat branch context as advisory for wording and scope, not as justification to include unrelated files.

2. Decide what to read in detail.
- Read full contents only for changed source, test, docs, config, or manifest files that are central to understanding the candidate commit.
- Prefer reading intent-carrying files first, such as `Cargo.toml`, `package.json`, `go.mod`, `README*`, module-local docs, and tests.
- Prefer targeted diff hunks over full-file reads when the change is localized.
- Avoid fully reading lockfiles and generated artifacts unless they are the main substance of the change or no better source exists.
- Be especially cautious with `package-lock.json`, `Cargo.lock`, `pnpm-lock.yaml`, `yarn.lock`, minified assets, vendored code, snapshots, and large machine-generated files.
- For noisy files, prefer filename presence, diff stats, and nearby manifest or source changes before deciding they need deeper inspection.
- When many files are touched, cluster by directory or subsystem before reading many files so you can decide whether this should be one commit or a split.
- If several changed files follow the same pattern, read 1-2 representative files fully and use diff/stat output for the rest unless the split decision depends on them.

3. If there are staged changes:
- Treat the staged set as the only candidate commit for this proposal.
- Inspect `git diff --staged`.
- Ignore unrelated unstaged or untracked files unless they are necessary to explain why the staged set is incomplete or suspicious.
- If the staged set is coherent, return `status="ready"` and set `stage_paths` to exactly the staged file list.
- If the staged set mixes concerns, return `status="split_required"` and explain the split options without proposing new staging changes.

4. If nothing is staged:
- Inspect the unstaged diff with `git diff`.
- Only inspect untracked files that plausibly belong to the same coherent change.
- If there is one unambiguous commit, return `status="ready"` with the repo-relative files for that commit in `stage_paths`.
- If changes should be split, return `status="split_required"` with concise alternative commit suggestions.
- If there is no meaningful change to commit, return `status="nothing_to_commit"`.

## Output Requirements
- `summary` should explain why the proposal is ready or why it stopped.
- Always include `stage_paths`, `commit`, and `alternatives` so the response matches the schema.
- When `status="ready"`, `commit.subject` must be a Conventional Commit style subject and `commit.body_paragraphs` should contain only meaningful paragraphs. Use an empty array when no body is needed.
- When `status` is not `ready`, set `commit` to `null`.
- When there is no single ready proposal, use `stage_paths: []`.
- When `status="split_required"`, populate `alternatives`. Otherwise use `alternatives: []`.

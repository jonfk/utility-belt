# Repository Guidelines

## Project Structure & Module Organization
- `go/` hosts CLI utilities in `src/github.com/jonfk/utility-belt/*`.
- `js/` contains the downloader client and Fastify server.
- `python/` stores standalone scripts plus `uv`-ready tools (`git-smart-commit`, `prune-openapi`).
- `rust/` tracks Cargo apps (`cmd-queue`, `move-photos-without-duplicates`).
- `firefox/video-downloader/` covers the browser extension; `sh/` holds reusable shell helpers.

## Build, Test, and Development Commands
- `just` shows helper installs; run `just install-…` targets for local symlinks.
- Go: inside each tool, run `go build ./...` or `go run .`; prefer `go test ./...` before release.
- Node/TS: `npm install`, `npm run dev` (tsx), `npm run build`, then `npm start` from `js/video-downloader-server`.
- Rust: `cargo run` for development and `cargo build --release` when packaging.
- Python: `uv run <script>.py` or activate a venv and `pip install -r python/requirements.txt`.

## Coding Style & Naming Conventions
- Go: enforce `gofmt`/`goimports`, CamelCase exports, and single-purpose packages.
- TypeScript: adhere to `tsconfig.json`, camelCase identifiers, and lint with ESLint-compatible rules when possible.
- Rust: run `cargo fmt` + `cargo clippy --all-targets`; keep modules snake_case with explicit error types.
- Python: stay PEP 8 compliant with snake_case modules; prefer `typing` annotations for new code.
- Shell: keep `#!/usr/bin/env bash`, lowercase file names, and `set -euo pipefail` in new scripts.

## Testing Guidelines
- Go tests live beside code (`*_test.go`); run `go test ./...` before publishing binaries.
- TypeScript tests are not yet wired; add Vitest or Jest suites, hook them to `npm test`, and mock Puppeteer calls.
- Rust: add unit tests next to modules and integration suites under `tests/`; execute `cargo test`.
- Python: favor `pytest` files near scripts, cover edge cases, and fake network calls.

## Commit & Pull Request Guidelines
- Follow Conventional Commits (`type(scope): summary`) as in `feat(justfile): …`.
- In PRs, mention linked issues, summarize behavior changes, and attach CLI output or screenshots when helpful.
- Note new dependencies, migration steps, and local `test`/`build` results in the PR body.

## Technologies & Preferred Libraries
- Go: prefer `cobra` for CLI parsing.
- TypeScript/Node: favor `fastify` for APIs, `@sinclair/typebox` for schemas, `tsx` + `tsc` for builds, and `esbuild` for bundling.
- Rust: use `clap` for CLI flags, `tokio` + `reqwest` for async I/O, and `thiserror` and `error-stack` for error handling.
- Python: lean on `requests` for HTTP and manage packaging with `uv` recipes defined in the root `justfile`.

# Repository Guidelines

This is a simple utility service, therefore dependencies, code and implementation should keep things as simple as possible with an eye to maintainability without modification over time.


## Project Structure & Module Organization
- `src/`: TypeScript source
  - `server.ts`: Fastify boot + shutdown
  - `routes.ts`: API routes (`/v1/*`, `/healthz`)
  - `schemas.ts`: TypeBox schemas + TS types
  - `services/`: `download.ts` (queue + downloader), `name.ts` (name resolver)
  - Infra: `puppeteer.ts`, `storage.ts`, `config.ts`, `error-handler.ts`, `errors.ts`
- `dist/`: Transpiled JS output (created by `tsc`)
- `data/`: Runtime download directory (set by `DATA_DIR`)
- Tooling: `tsconfig.json`, `Dockerfile`, `justfile`, `package.json`

## Build, Test, and Development Commands
- `npm run dev`: Start Fastify with tsx (hot TypeScript).
- `npm run build`: Compile TypeScript to `dist/`.
- `npm start`: Run compiled server from `dist/server.js`.
- `just dev|build|start`: Shorthand wrappers for the above.
- Docker:
  - Build: `just docker-build` or `docker build -t video-downloader-server .`
  - Run: `just docker-run` or `docker run -p 3000:3000 video-downloader-server`

Note: `npm test` is a placeholder and currently exits non‑zero (no tests yet).

## Coding Style & Naming Conventions
- Language: TypeScript (strict), ESNext modules.
- Indentation: prefer 2 spaces; stay consistent within touched files.
- Quotes & semicolons: single quotes, semicolons present—match existing files.
- Naming: kebab‑case filenames (e.g., `error-handler.ts`); `PascalCase` for types/interfaces; `camelCase` for variables/functions; named exports over default when reasonable.
- Lint/format: no config committed—avoid broad reformatting; limit diffs to your change.
- Dependencies: don't modify package.json directly, install with `npm install` unless specifically directed otherwise.

## Commit & Pull Request Guidelines
- Commits: follow Conventional Commits observed in history, e.g. `feat(scope): …`, `refactor(download): …`, `chore(justfile): …`, `build(video-downloader-server): …`, `docs(server): …`.
- PRs: small and focused; include:
  - What/why, linked issues, and any config/env changes.
  - Local run steps and sample requests (e.g., `curl -X POST :3000/v1/download …`).
  - Logs or screenshots when changing Docker/runtime behavior.

## Security & Configuration Tips
- Env vars: `PORT` (default 3000), `DATA_DIR` (default `./data`), `REQUEST_TIMEOUT_MS`.
- Puppeteer runs headless with `--no-sandbox`/`--disable-dev-shm-usage`; deploy on trusted networks only.
- Avoid logging secrets or tokenized URLs; keep error details concise in responses.


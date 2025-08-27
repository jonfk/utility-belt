Here’s a simple, stable, TypeScript + Fastify design that keeps moving parts to a minimum while leaving clean seams for your Puppeteer-specific logic later.

## Goals & Design Principles

- Simplicity first: a single Fastify app, minimal plugins, one browser instance reused across requests. Fastify’s plugin model keeps concerns isolated and maintainable.
- Stability over time: use JSON Schema for request/response validation (native Fastify path) with TypeBox for TS types, so types and validation stay in sync.
- Predictable headless runtime: stick to Puppeteer’s bundled “Chrome for Testing” for API compatibility over time.
- Sequential downloads: maintain an in-memory FIFO queue; process one job at a time.
- Async API: `POST /v1/download` enqueues and returns immediately with a `jobId`.

## High-Level Architecture

### Layers
1. HTTP API (Fastify) — routing + validation
2. Services
   - `services/name.ts`: NameResolver (split out, swappable)
   - `services/download.ts`: tiny FIFO queue + in-memory completed list + per-domain downloader dispatch
3. Infrastructure (flattened files)
   - `puppeteer.ts`: singleton Browser + helpers
   - `storage.ts`: `DATA_DIR`, path safety, fs ops
   - `config.ts`: env + defaults

Per-domain strategies
- Resolver selection: pick a `NameResolver` based on URL/hostname pattern (e.g., YouTube-specific rules).
- Downloader selection: pick a downloader implementation per URL/hostname; an implementation may use Puppeteer or a non-Puppeteer approach.

### Endpoints (v1)
- POST `/v1/name` — body `{ url }` → `{ name }` (resolver chosen by URL/domain)
- POST `/v1/download` — body `{ url, name }` → `{ jobId, status: "enqueued" }`
- GET `/v1/downloads/completed` — list completed downloads kept in memory
- GET `/healthz` — readiness/liveness

## Why This Way

- Fastify + JSON Schema: fast validation and stable contracts.
- Async enqueue keeps requests fast; a single-worker queue ensures predictable resource use.
- One reused browser instance for strategies that need it; decoupled so other strategies can skip Puppeteer entirely.
- Encapsulation via small modules keeps the app small and easy to reason about.

## API Schemas

Use TypeBox for schemas and types (Fastify’s docs recommend it):

- POST `/v1/name`
  - Request
    ```json
    { "url": "https://example.com" }
    ```
  - Response
    ```json
    { "name": "derived-name" }
    ```
- POST `/v1/download`
  - Request
    ```json
    { "url": "https://example.com/file", "name": "my-file" }
    ```
  - Response
    ```json
    {
      "jobId": "a1b2c3d4",
      "status": "enqueued"
    }
    ```

- GET `/v1/downloads/completed`
  - Response
    ```json
    [
      {
        "url": "https://example.com/file",
        "name": "my-file",
        "savedPath": "data/my-file.ext",
        "size": 123456,
        "startedAt": "2024-01-01T00:00:00.000Z",
        "finishedAt": "2024-01-01T00:00:05.000Z"
      }
    ]
    ```

Fastify’s TypeScript guide shows how to wire TypeBox so the same schema powers both runtime validation and compile-time types.

See https://fastify.dev/docs/latest/Reference/Type-Providers/#typebox

## Puppeteer Infra (optional)

- Lifecycle: create one Browser on startup, reuse it; strategies that use Puppeteer create a Page per job and close it after. Keeps memory predictable.

Why reuse the browser? Lower cold-start latency and memory churn. Not all downloaders need Puppeteer—some may use HTTP or other methods.

## Graceful Startup & Shutdown

- Startup: launch the browser once; register Fastify routes; log config.
- Shutdown: on SIGINT/SIGTERM, call `fastify.close()` and then close the Puppeteer browser in an `onClose` hook. This lets in-flight requests finish cleanly.
- Optional helpers like `fastify-graceful-shutdown` can be added later; `fastify.close()` + hooks is sufficient here.

## Security

- Since this is a utility server used mainly by the creator of the service, no authentication or other security is needed.
- Keep it simple and minimal. Assume only privileged users can access it.

## Configuration (env-driven)

- `PORT` (default 3000)
- `DATA_DIR` (where files land; default `./data`)
- `REQUEST_TIMEOUT_MS` (Fastify + Puppeteer job timeout)
- `HEADLESS=true|false` (Puppeteer launch flag; default headless)

Puppeteer pins a compatible Chrome build at install time, aiding long-term stability; keep Node at LTS and avoid bleeding-edge flags.

## Minimal Project Structure

```
src/
├─ server.ts          # boot & shutdown
├─ routes.ts          # registers endpoints
├─ schemas.ts         # TypeBox schemas
├─ puppeteer.ts       # singleton Browser + helpers
├─ storage.ts         # DATA_DIR, path safety, fs ops
├─ config.ts          # env + defaults
└─ services/
   ├─ name.ts         # NameResolver (split out)
   └─ download.ts     # tiny FIFO queue + completed list + strategy dispatch
```

## Runtime Notes & Best Practices

- Validation: JSON Schema + TypeBox (officially recommended pattern) keeps you fast and typed.
- Utilities: `@fastify/sensible` gives small helpers (e.g., `httpErrors`) without bloating code.
- Ecosystem: if you later want OpenAPI or graceful exit helpers, the Fastify ecosystem has drop-in plugins—optional for now to keep things simple.

## Download Flow (Implementation Sketch)

1. Route handler enqueues and returns
   - `POST /v1/download` validates input, enqueues a job, and returns `{ jobId, status: "enqueued" }` immediately.
2. Worker processes jobs sequentially
   - Validate `url`; ensure `name` maps to a safe path under `DATA_DIR`.
   - Resolve strategy by URL/hostname: pick the appropriate downloader implementation.
   - Run the downloader implementation; it may use `puppeteer.ts` or perform non-Puppeteer work as needed.
   - On success, append a record to the completed list `{ url, name, savedPath, size, startedAt, finishedAt }`.

## Job Queue & Completed List

- Queue: implemented locally in `services/download.ts` by chaining promises; only one job runs at a time. If the process restarts, the queue is empty.
- Completed list: in-memory array in `services/download.ts` capturing `{ url, name, savedPath, size, startedAt, finishedAt }`. Exposed via `GET /v1/downloads/completed`.

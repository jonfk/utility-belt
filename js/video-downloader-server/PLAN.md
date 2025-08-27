Here’s a simple, stable, TypeScript + Fastify design that keeps moving parts to a minimum while leaving clean seams for your Puppeteer-specific logic later.

## Goals & Design Principles

- Simplicity first: a single Fastify app, minimal plugins, one browser instance reused across requests. Fastify’s plugin model keeps concerns isolated and maintainable.
- Stability over time: use JSON Schema for request/response validation (native Fastify path) with TypeBox for TS types, so types and validation stay in sync.
- Predictable headless runtime: stick to Puppeteer’s bundled “Chrome for Testing” for API compatibility over time.
- Sequential downloads: maintain an in-memory FIFO queue; process one job at a time.

## High-Level Architecture

### Layers
1. HTTP API (Fastify) — routing + validation
2. Services
   - NameResolver (interface): given a URL → returns a name (implementation pluggable)
   - Downloader (service): given `{ url, name }` → drives Puppeteer to download into a target folder
3. Infrastructure
   - PuppeteerManager: singleton owning one Browser, creates/cleans Page per job, handles graceful shutdown
   - Storage: safe file/path handling under a configured `DATA_DIR`
   - JobQueue (sequential): in-memory FIFO queue with a single worker that runs one download at a time
   - CompletedStore: in-memory list of completed downloads (for a simple read API)

### Endpoints (v1)
- POST `/v1/name` — body `{ url }` → `{ name }`
- POST `/v1/download` — body `{ url, name }` → `{ savedPath, size, startedAt, finishedAt }`
- GET `/v1/downloads/completed` — list completed downloads kept in memory
- GET `/healthz` — readiness/liveness

## Why This Way

- Fastify + JSON Schema: fast validation and stable contracts.
- One reused browser instance, new page per task: lower memory and fewer crashes than per-request launches; cap parallel pages.
- Encapsulation via plugins keeps the app small and easy to reason about.

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
      "savedPath": "data/my-file.ext",
      "size": 123456,
      "startedAt": "2024-01-01T00:00:00.000Z",
      "finishedAt": "2024-01-01T00:00:05.000Z"
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

## Puppeteer Strategy

- Lifecycle: create one Browser on startup, reuse it; create a new Page per request, close it after. Keeps memory predictable.

Why reuse the browser? Lower cold-start latency and memory churn; widely recommended to reuse a single browser and limit parallel pages.

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
- No `CONCURRENCY`: downloads run strictly sequentially via the in-memory queue
- `REQUEST_TIMEOUT_MS` (Fastify + Puppeteer job timeout)
- `ALLOWED_HOSTS` (comma-separated allowlist)
- `HEADLESS=true|false` (Puppeteer launch flag; default headless)

Puppeteer pins a compatible Chrome build at install time, aiding long-term stability; keep Node at LTS and avoid bleeding-edge flags.

## Minimal Project Structure

```
fastify-puppeteer-downloader/
├─ src/
│  ├─ app.ts                 # build Fastify instance (register routes/plugins)
│  ├─ server.ts              # boot: create app, start listen, wire signals
│  ├─ routes/
│  │  ├─ name.routes.ts      # POST /v1/name
│  │  └─ download.routes.ts  # POST /v1/download
│  ├─ services/
│  │  ├─ NameResolver.ts     # interface + default stub
│  │  └─ Downloader.ts       # uses PuppeteerManager + Storage
│  ├─ infra/
│  │  ├─ PuppeteerManager.ts # singleton Browser, page factory, shutdown
│  │  ├─ Storage.ts          # safe filename/path utils, fs ops
│  │  ├─ JobQueue.ts         # sequential in-memory FIFO queue, single worker
│  │  └─ CompletedStore.ts   # in-memory list of completed download records
│  ├─ schemas/
│  │  ├─ name.ts             # TypeBox schemas (req/res)
│  │  └─ download.ts
│  └─ config.ts              # env parsing (with defaults)
├─ data/                     # download root (gitignored)
├─ test/                     # unit + light integration tests
├─ package.json
└─ tsconfig.json
```

## Runtime Notes & Best Practices

- Validation: JSON Schema + TypeBox (officially recommended pattern) keeps you fast and typed.
- Utilities: `@fastify/sensible` gives small helpers (e.g., `httpErrors`) without bloating code.
- Ecosystem: if you later want OpenAPI or graceful exit helpers, the Fastify ecosystem has drop-in plugins—optional for now to keep things simple.

## Download Flow (Implementation Sketch)

1. `Downloader.download({ url, name })`
   - Validate `url` against allowlist and scheme; normalize `name` and ensure the final path is within `DATA_DIR` (SSRF and traversal safeguards).
   - Enqueue the job into `JobQueue`; only one job runs at a time. The request handler awaits the job’s completion, preserving the existing synchronous API shape.
   - Get browser from `PuppeteerManager`, then `const page = await browser.newPage()`.
   - Configure download directory via CDP (`Browser.setDownloadBehavior` when available; otherwise `Page.setDownloadBehavior` via `CDPSession`), trigger the click/navigation that starts the download; wait until completion (CDP event or directory polling).
   - Close the page, resolve the job, update `CompletedStore`, return `{ savedPath, size, startedAt, finishedAt }`.

## Job Queue & Completed List

- JobQueue: a tiny in-memory FIFO with a single worker; each `enqueue(fn)` chains onto a promise and runs only after the previous job resolves. If the process restarts, the queue is naturally empty.
- CompletedStore: append-only list in memory capturing `{ url, name, savedPath, size, startedAt, finishedAt }` for successful jobs. Exposed via `GET /v1/downloads/completed`.

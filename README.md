# AI-Driven Website

A personal-resume website where the visual style is generated on demand by an LLM. The page content (about, experience, projects) is served from a small Rust/Rocket backend, and the styles are produced by Anthropic Claude (on Azure) from a theme prompt + the rendered HTML. A "Download Resume" button generates a PDF on the fly from the same data using Typst.

This repo is a replacement of my [personal website](https://github.com/holmes-software/personal-website).

## Project layout

```
backend/                  Rust + Rocket API
  src/                    main, profile, styles (cache + LLM), resume (PDF), CORS
  data/profile.json       Static profile data served at GET /api/profile
  data/images/            Profile pic & project images (served via Rocket FileServer)
  templates/resume.typ    Typst template used by GET /api/resume.pdf
  cache/                  Disk-backed cache fallback: <datetime>-<theme>-styles.css
frontend/                 Vite + TypeScript (no framework)
  index.html              Semantic structure for the LLM to style
  src/main.ts             Fetch data, render content, request styles, download PDF
```

## API

| Method | Path | Description |
| --- | --- | --- |
| GET | `/api/profile` | Returns the JSON in `backend/data/profile.json`. |
| GET | `/api/images/<path>` | Serves files from `backend/data/images/` via Rocket's `FileServer`. |
| GET | `/api/resume.pdf` | Renders the resume PDF via Typst from `profile.json`. |
| POST | `/api/styles` | Body `{ "html": string, "theme": string }`. Returns `{ "css": string, "theme": string, "source": "cache"\|"llm"\|"fallback" }`. |

### Theme slug

`theme` is validated server-side as `^[a-z0-9]{1,10}$` (lowercase alphanumeric, single token, max 10 chars). The frontend mirrors this normalization. Rationale:

- **Prompt-injection surface.** The slug is interpolated into the LLM prompt; restricting the alphabet defangs it as an instruction-injection vector.
- **Cache-key safety.** The slug becomes part of disk filenames and Redis keys; the alphabet keeps it filesystem- and Redis-safe and forecloses path traversal.
- **Cache-cardinality cap.** Per-theme caching invites enumeration; the length cap bounds growth.

Note: the bigger surface is the LLM-generated CSS itself (which can `@import`, `url(...)`, fingerprint via selectors, etc.). A Content-Security-Policy is the right defense for that and is recommended as a follow-up.

### Caching

Two backends, selected at startup:

- **Azure Cache for Redis** when `REDIS_URL` is set, e.g. `rediss://:<access-key>@<name>.redis.cache.windows.net:6380`. Keys are `styles:<theme>` with a TTL of `CACHE_TTL_MINUTES`.
- **Disk** (default fallback) — files at `backend/cache/<datetime>-<theme>-styles.css`. On lookup, the most recent non-stale file matching the requested theme wins.

Only successful LLM responses are cached; built-in fallbacks are not (so a transient outage doesn't poison the cache).

### LLM provider

The styles endpoint calls **Microsoft Foundry** via its OpenAI-compatible chat-completions API. The model is selected on the Foundry side. If the call fails (or no credentials are set), the endpoint returns a built-in default stylesheet so the site still works locally.

- `AZURE_FOUNDRY_URL` — full request URL for your Foundry deployment's chat-completions endpoint.
- `AZURE_FOUNDRY_API_KEY` — API key for the deployment.

## Running locally

This repo uses [`just`](https://github.com/casey/just) as the task runner.

| Command | Description |
| --- | --- |
| `just run` | Runs backend + frontend dev servers in parallel; Ctrl-C kills both. |
| `just test` | Runs backend tests (`cargo test`) then frontend tests (`vitest run`). |
| `just build` | `cargo build --release` + `npm run build`. |
| `just fmt` | `cargo fmt` + `prettier --write`. |
| `just fmt-check` | `cargo fmt -- --check` + `prettier --check`. |

Backend listens on `:8000`, Vite on `:5173` (proxies `/api` → backend). Open http://localhost:5173.


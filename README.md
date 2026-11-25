# DataFulton

> [!WARNING]
> **Untested software — use at your own risk.** DataFulton is built from publicly available API documentation and has **not been tested against any of the included platform APIs** (Prime Eco, Procore, BuildPass, Simpro, AroFlo, Monday.com). It has also **not been tested on Windows**. While the client is strictly read-only (GET-only; it cannot modify your account), verify exported data against your platform before relying on it. Provided as-is with no warranty — see LICENSE.

A small cross-platform desktop app (macOS + Windows) that **extracts your own account data**
from construction & field-service platforms and saves it locally — one JSONL file per entity,
plus downloaded attachments.

Providers are pluggable: **Prime Eco, Procore, BuildPass, Simpro, AroFlo, Monday.com** ship
today, each driven by a provider spec (host, auth, entity/child paths) — see
`src/renderer/src/providers/`.

Built with [Tauri](https://tauri.app) (Rust core + web UI). ~15 MB binary, native performance.

## The read-only guarantee

DataFulton **cannot modify your account**. The client only implements `GET` (plus `POST` where a
platform exposes data exclusively through a query endpoint — e.g. Monday.com GraphQL). There is
no PUT/PATCH/DELETE code anywhere. It exists purely to let you download a copy of your own data.

## Connections & sessions

DataFulton is organized around **connections**: each connection is a platform + host +
credential — e.g. a bearer API key for `{provider}` at `https://{host}/api`, or OAuth
client-credentials for platforms that issue tokens (`/oauth/token`). All extraction **sessions**
belong to a connection.

Getting credentials:

1. **Manual (recommended):** sign in to your `{provider}` account → account/developer settings →
   **API** / **API Tokens** / **Developer** → create & copy a token (or OAuth client id/secret).
2. **Auto-detect:** on a saved connection, click *"Find my key automatically"* — a window
   opens to the site; sign in and DataFulton will try to spot the token in site storage.

Credentials are stored **only in the app's local SQLite database** (inside the app data
directory) — no OS keychain, no cloud. They never leave your machine except as auth headers on
requests to your own `{provider}` host.

## Using it

1. Create a **connection** — pick a platform, name + host + credentials, tested on save
2. **+ New extraction** — pick a connection, then data types (jobs, contacts, estimates,
   invoices…), optionally:
   - **Per-record details** — children like notes/histories/schedules (one request per record)
   - **Download attached files** — binaries fetched via each record's `url` field
3. Pick destination + speed:
   - **Safe**: ~25 req/min, 1 concurrent — gentle on the API
   - **Aggressive**: ~150 req/min, 4 concurrent — auto-backs-off on HTTP 429

Sessions survive crashes and restarts — every page is checkpointed to a local SQLite database;
interrupted sessions show up as *paused* on next launch and resume where they left off.
File downloads resume mid-file via HTTP `Range` when supported.

## Output layout

```
<destination>/{provider}-export-<timestamp>-<id>/
  manifest.json            # session summary: counts, statuses, errors
  data/<entity>.jsonl      # one JSON record per line (raw API objects)
  data/<child>.jsonl       # per-record records carry _parentType/_parentId
  files/<entity>/…         # downloaded binaries
```

## Development

Requires: Node 20+, Rust (stable), and on macOS the Xcode command line tools.

```bash
npm install
npm run dev                              # vite dev server + tauri dev window
node scripts/test-servers.mjs            # local mock APIs for all providers on :5174
node scripts/test-servers.mjs --platforms primeeco,procore  # only some
npm test                                 # engine integration test
npm run typecheck                        # renderer types
npm run ui:build                         # build just the frontend
```

The test servers print a fake API key + OAuth pair at startup — point a connection at
`http://localhost:5174/api/{provider}/v2` to exercise the full flow without real credentials.

## Building installers

```bash
npm run build      # produces native bundles in src-tauri/target/release/bundle/
```

- **macOS** → `.app`, `.dmg` (run on a Mac)
- **Windows** → `.msi`/`.exe` (run on a Windows machine or CI — cross-compiling
  Windows bundles from macOS isn't supported by Tauri)

Unsigned builds work fine locally; for distribution you'll want to sign/notarize
(see `tauri.conf.json` → `bundle`).

### Running an unsigned macOS download

Releases ship a universal `.dmg` (Apple Silicon + Intel) but the app is **not
code-signed**, so macOS will quarantine it and report "damaged and can't be
opened". After copying to `/Applications`, clear the quarantine flag:

```bash
xattr -dr com.apple.quarantine /Applications/DataFulton.app
```

or right-click the app → **Open** → **Open** anyway. To distribute without this
step you'd need an Apple Developer Program signing certificate.

## Notes on provider APIs

- Auth: `Authorization: Bearer <key>` (bearer providers) or client-credentials →
  bearer token (OAuth providers); AroFlo uses HMAC signing
- Pagination: `page` + `per_page`, `meta.last_page`/`total` where the API exposes it
- Filtering: `q='field'.eq('value'),…`-style or provider-specific params
- Specs reconstructed from publicly available API documentation

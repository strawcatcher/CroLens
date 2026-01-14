# crolens-api

Rust + Cloudflare Workers backend for CroLens (MCP-compatible JSON-RPC 2.0 API) on Cronos.

## HTTP endpoints

- `POST /` - JSON-RPC 2.0 (`tools/list`, `tools/call`)
- `GET /health` - service health
- `GET /stats` - lightweight stats for the frontend (e.g. `protocols_supported`)
- `GET /x402/quote` - fetch top-up quote (amount, payment address, credits)
- `GET /x402/status` - check current API key tier + credits (requires `x-api-key`)
- `POST /x402/verify` - verify a payment tx and grant credits (requires `x-api-key`)

## Local development

### Prerequisites

- Rust 1.75+ with `wasm32-unknown-unknown` target
- Node.js 18+
- Wrangler CLI (`npm install -g wrangler`)

### Setup

```bash
cd crolens-api
npm install

cp .dev.vars.example .dev.vars
# Fill required env vars (see below)

wrangler d1 execute crolens-db --local --file=./db/schema.sql
wrangler d1 execute crolens-db --local --file=./db/seed.sql

wrangler dev --local --persist-to .wrangler/state
```

## Environment variables

Required:

- `BLOCKPI_RPC_URL` - Cronos RPC endpoint for reads (eth_call, tx lookup)

Optional:

- `RPC_MAX_RETRIES` - defaults to `3`
- `RPC_TIMEOUT_MS` - request timeout in milliseconds, defaults to `10000`
- `RPC_CACHE_TTL_SECS` - caches successful RPC responses in KV, defaults to `300`
- `TENDERLY_ACCESS_KEY` / `TENDERLY_API_KEY`, `TENDERLY_ACCOUNT`, `TENDERLY_PROJECT` - enable `simulate_transaction` and swap simulation guard
- `X402_PAYMENT_ADDRESS` - enable x402 top-up flow (Console + `/x402/*` endpoints)
- `X402_TOPUP_CREDITS` - defaults to `1000`
- `CORS_ALLOW_ORIGIN` - comma-separated allowlist; use `*` to allow all; empty denies browser origins (403)
- `REQUEST_LOG_SAMPLE_RATE` - sample successful tool calls (0..1), defaults to `1.0`
- `RATE_LIMIT_JSONRPC_PER_MIN` - per-IP rate limit for `POST /` JSON-RPC requests, defaults to `120`
- `RATE_LIMIT_JSONRPC_WINDOW_SECS` - rate limit window in seconds, defaults to `60`

## Notes

- Anchor prices are refreshed every 5 minutes via Worker cron and stored in KV.
- Non-anchor token prices are derived from VVS pools and cached in KV (`price:derived:{address}`).
- Tool calls are logged into D1 `request_logs` for debugging and dashboard correlation via `trace_id`.

## Deployment

```bash
cd crolens-api
wrangler deploy
```

For production secrets, prefer `wrangler secret put ...` or Cloudflare Dashboard secrets.

### Production config

Use `wrangler.production.toml` for production deploys:

```bash
wrangler deploy -c wrangler.production.toml
```

Required secrets (production):

- `BLOCKPI_RPC_URL`
- `TENDERLY_ACCESS_KEY`, `TENDERLY_ACCOUNT`, `TENDERLY_PROJECT` (optional but recommended for simulation)

If you already have an existing D1 database, apply the one-time schema migration:

```bash
wrangler d1 execute crolens-db --remote --file=./db/migrate_request_logs_columns.sql
```

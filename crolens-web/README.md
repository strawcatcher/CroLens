# crolens-web

React + Vite frontend for CroLens. Includes:

- **Playground**: test all MCP tools (`tools/call`) with formatted output + raw JSON
- **Dashboard**: session-level metrics + live logs
- **Console**: API key + credits + x402 top-up flow

## Local development

### Prerequisites

- Node.js 18+

### Setup

```bash
cd crolens-web
npm install

cp .env.example .env
# Update VITE_API_URL if your backend is not running at http://localhost:8787

npm run dev
```

## Environment variables

- `VITE_API_URL` - backend base URL (required in production; defaults to `http://localhost:8787` in dev)
- `VITE_WALLETCONNECT_PROJECT_ID` - optional (enables WalletConnect connector in RainbowKit)
- `VITE_SENTRY_DSN` - optional (enables Sentry reporting for errors captured by `src/lib/monitoring.ts`)
- `VITE_SENTRY_ENVIRONMENT` - optional (defaults to Vite `MODE`)

## Build

```bash
npm run build
npm run preview
```

## Notes

- The Console page uses backend x402 endpoints (`/x402/quote`, `/x402/verify`). Make sure the backend is configured with `X402_PAYMENT_ADDRESS` for real top-ups.

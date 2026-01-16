# CroLens

> **Making Cronos Readable for Machines** - Semantic data layer for AI Agents on Cronos blockchain.

CroLens transforms complex blockchain data into structured, AI-friendly responses. One API call returns semantic data that would otherwise require RPC calls + ABI decoding + context understanding + formatting.

## Features

- **MCP Protocol Compatible** - JSON-RPC 2.0 interface for AI Agents and LLM applications
- **Cronos Native** - Deep integration with VVS Finance and Tectonic Protocol
- **Semantic Translation** - Raw blockchain data → human-readable summaries
- **Transaction Builder** - Construct swap transactions with built-in simulation guard
- **Edge Performance** - Rust + Cloudflare Workers for low-latency responses

## MCP Tools

| Tool | Description |
|------|-------------|
| `get_account_summary` | Complete account overview: wallet balances + DeFi positions |
| `get_defi_positions` | Detailed DeFi positions (VVS LP, Tectonic supply/borrow) |
| `get_token_info` | Query token metadata (name, symbol, decimals, total supply) |
| `get_token_price` | Batch query token prices via VVS pools |
| `get_pool_info` | Query VVS LP pool details and reserves |
| `get_gas_price` | Get current gas price in gwei/CRO |
| `get_block_info` | Get block details by number or 'latest' |
| `get_approval_status` | Check ERC20 token allowance for spender |
| `decode_transaction` | Translate transaction hash to human-readable action |
| `simulate_transaction` | Simulate transaction execution with state changes |
| `search_contract` | Search contracts by name, symbol, or address |
| `construct_swap_tx` | Build swap calldata with approval handling |

## Quick Start

### MCP Client Integration

Add to your MCP client configuration:

```json
{
  "mcpServers": {
    "crolens": {
      "command": "npx",
      "args": ["-y", "crolens-mcp"],
      "env": {
        "CROLENS_API_KEY": "your_api_key_here"
      }
    }
  }
}
```

### Direct API Usage

```bash
curl -X POST https://crolens-api.crolens.workers.dev \
  -H "Content-Type: application/json" \
  -H "x-api-key: cl_sk_your_key" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "get_account_summary",
      "arguments": {
        "address": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23",
        "simple_mode": true
      }
    }
  }'
```

## Project Structure

```
crolens/
├── crolens-api/          # Rust backend (Cloudflare Worker)
│   ├── src/
│   │   ├── lib.rs        # Entry point + Cron handler
│   │   ├── error.rs      # Unified error types
│   │   ├── gateway/      # Auth + x402 payment
│   │   ├── mcp/          # MCP protocol (router, tools)
│   │   ├── domain/       # Business logic (assets, defi, simulation)
│   │   ├── adapters/     # Protocol adapters (uniswap_v2, compound_v2)
│   │   └── infra/        # Infrastructure (rpc, multicall, price)
│   ├── db/
│   │   ├── schema.sql    # D1 database schema
│   │   └── seed.sql      # Initial data
│   └── wrangler.toml     # Cloudflare config
│
└── crolens-web/          # React frontend
    └── src/
        ├── components/   # UI components (shadcn/ui)
        ├── features/     # Playground, Dashboard, Console
        ├── lib/          # API client, wagmi config
        └── stores/       # Zustand stores
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| Backend | Rust + Cloudflare Workers + alloy-rs |
| Frontend | React + Vite + shadcn/ui + Tailwind |
| Storage | Cloudflare KV (cache) + D1 (database) |
| Web3 | RainbowKit + Wagmi |
| RPC | BlockPi (query) + Tenderly (simulation) |

## Development

### Prerequisites

- Rust 1.75+ with `wasm32-unknown-unknown` target
- Node.js 18+
- Wrangler CLI (`npm install -g wrangler`)
- Cloudflare account

### Backend Setup

```bash
cd crolens-api
npm install

# Local env vars (recommended for wrangler --local)
cp .dev.vars.example .dev.vars

# Configure secrets (for remote deploy)
wrangler secret put BLOCKPI_RPC_URL
wrangler secret put TENDERLY_ACCESS_KEY
wrangler secret put TENDERLY_ACCOUNT
wrangler secret put TENDERLY_PROJECT

# Initialize database
wrangler d1 execute crolens-db --local --file=./db/schema.sql
wrangler d1 execute crolens-db --local --file=./db/seed.sql

# Run locally
wrangler dev --local --persist-to .wrangler/state
```

### Frontend Setup

```bash
cd crolens-web
npm install
npm run dev
```

### Deployment

```bash
# Deploy backend
cd crolens-api
wrangler deploy

# Deploy frontend
cd crolens-web
npm run build
# Deploy to your preferred hosting (Cloudflare Pages, Vercel, etc.)
```

## Supported Protocols

| Protocol | Type | Adapter |
|----------|------|---------|
| VVS Finance | DEX (Uniswap V2 fork) | `UniswapV2Adapter` |
| Tectonic | Lending (Compound V2 fork) | `CompoundV2Adapter` |

## Contract Addresses (Cronos Mainnet)

| Contract | Address |
|----------|---------|
| Multicall3 | `0xcA11bde05977b3631167028862bE2a173976CA11` |
| WCRO | `0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23` |
| USDC | `0xc21223249CA28397B4B6541dfFaEcC539BfF0c59` |
| VVS Router | `0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae` |
| VVS Factory | `0x3B44B2a187a7b3824131F8db5a74194D0a42Fc15` |
| TectonicCore | `0x7De56Bd8b37827c51835e162c867848fE2403a48` |

## Contributing

### Code Style

- **Language**: All code, comments, and commits MUST be in English
- **Rust**: Follow standard Rust conventions (`cargo fmt`, `cargo clippy`)
- **TypeScript**: ESLint + Prettier
- **Commits**: Use [Conventional Commits](https://www.conventionalcommits.org/)
  - `feat:` new feature
  - `fix:` bug fix
  - `docs:` documentation
  - `refactor:` code refactoring
  - `test:` adding tests
  - `chore:` maintenance

### Commit Message Examples

```
feat(mcp): add get_account_summary tool
fix(rpc): handle timeout in multicall aggregation
docs: update API usage examples
refactor(adapters): extract common UniswapV2 logic
```

## License

MIT

## Acknowledgments

Built for [Cronos Hackathon 2025](https://cronos.org/)

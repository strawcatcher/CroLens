<div align="center">

# ◆ CROLENS ◆

### 👁️ THE EYES OF AI AGENTS ON CRONOS

**Read-Only. No Private Keys. Zero Risk.**

[![Cronos](https://img.shields.io/badge/Cronos-002D74?style=for-the-badge&logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMzIiIGhlaWdodD0iMzIiIHZpZXdCb3g9IjAgMCAzMiAzMiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxNiIgY3k9IjE2IiByPSIxNiIgZmlsbD0id2hpdGUiLz48L3N2Zz4=&logoColor=white)](https://cronos.org/)
[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP_Protocol-FF6B35?style=for-the-badge&logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMzIiIGhlaWdodD0iMzIiIHZpZXdCb3g9IjAgMCAzMiAzMiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxNiIgY3k9IjE2IiByPSIxNiIgZmlsbD0id2hpdGUiLz48L3N2Zz4=&logoColor=white)](https://modelcontextprotocol.io/)
[![Cloudflare Workers](https://img.shields.io/badge/Cloudflare_Workers-F38020?style=for-the-badge&logo=cloudflare&logoColor=white)](https://workers.cloudflare.com/)

[![Live Demo](https://img.shields.io/badge/🔴_LIVE_DEMO-crolens--web.pages.dev-D90018?style=for-the-badge)](https://crolens-web.pages.dev)
[![API Status](https://img.shields.io/badge/API-ONLINE-00FF41?style=for-the-badge)](https://crolens-api.crolens.workers.dev)

<br/>

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   ██████╗██████╗  ██████╗ ██╗     ███████╗███╗   ██╗███████╗│
│  ██╔════╝██╔══██╗██╔═══██╗██║     ██╔════╝████╗  ██║██╔════╝│
│  ██║     ██████╔╝██║   ██║██║     █████╗  ██╔██╗ ██║███████╗│
│  ██║     ██╔══██╗██║   ██║██║     ██╔══╝  ██║╚██╗██║╚════██║│
│  ╚██████╗██║  ██║╚██████╔╝███████╗███████╗██║ ╚████║███████║│
│   ╚═════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚══════╝╚═╝  ╚═══╝╚══════╝│
│                                                             │
│          MAKING CRONOS READABLE FOR MACHINES                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

</div>

---

## 🎯 What is CroLens?

> **CroLens is the semantic data layer for AI Agents on Cronos blockchain.**

AI Agents need to **see** blockchain data, not **control** wallets. CroLens provides **read-only** access to on-chain data through the MCP protocol - no private keys required, zero security risk.

<table>
<tr>
<td width="50%">

### ❌ Traditional Way
```
1. RPC Call (raw hex data)
2. ABI Decoding (manual)
3. Context Understanding (hard)
4. Format Output (tedious)
```

</td>
<td width="50%">

### ✅ CroLens Way
```json
{
  "method": "get_account_summary",
  "params": { "address": "0x..." }
}
// → Semantic JSON response
```

</td>
</tr>
</table>

---

## 👁️ The "Eyes" Philosophy

<div align="center">

| 🔐 Traditional AI Wallet | 👁️ CroLens Approach |
|:---:|:---:|
| Needs private keys | **No keys required** |
| Can execute transactions | **Read-only by design** |
| Security risk | **Zero risk** |
| Complex setup | **One API call** |

</div>

> 💡 **CroLens is to AI Agents what read-only database access is to applications.**
>
> Your AI can see everything, but can't touch anything.

---

## ⚡ 30 MCP Tools

<div align="center">

[![Tools](https://img.shields.io/badge/MCP_TOOLS-30-D90018?style=for-the-badge)]()
[![VVS Finance](https://img.shields.io/badge/VVS_Finance-Integrated-7B3FE4?style=for-the-badge)]()
[![Tectonic](https://img.shields.io/badge/Tectonic-Integrated-00D1FF?style=for-the-badge)]()

</div>

### 📊 Account & Assets
| Tool | Description |
|------|-------------|
| `get_account_summary` | Complete wallet overview with DeFi positions |
| `get_defi_positions` | VVS LP + Tectonic supply/borrow details |
| `get_token_info` | Token metadata, price, liquidity |
| `get_token_price` | Batch price query (up to 20 tokens) |
| `get_portfolio_analysis` | Holdings analysis & diversification score |

### 🔄 DeFi Protocol Tools
| Tool | Description |
|------|-------------|
| `get_pool_info` | VVS LP pool details and reserves |
| `get_vvs_farms` | All VVS farms with TVL & APY |
| `get_vvs_rewards` | Pending VVS rewards per farm |
| `get_tectonic_markets` | All lending markets overview |
| `get_tectonic_rates` | Supply/borrow APY comparison |
| `get_lending_rates` | Cross-protocol rate comparison |
| `get_best_swap_route` | Optimal swap path across DEXs |

### 🔍 Transaction Intelligence
| Tool | Description |
|------|-------------|
| `decode_transaction` | Human-readable transaction summary |
| `decode_calldata` | Method signature + parameter parsing |
| `simulate_transaction` | Preview state changes + risk warnings |
| `estimate_gas` | Precise gas estimation in CRO/USD |

### 🛡️ Security & Monitoring
| Tool | Description |
|------|-------------|
| `get_approval_status` | Token allowances with risk scoring |
| `get_token_approvals` | Complete approval list per address |
| `get_liquidation_risk` | Health factor & liquidation threshold |
| `get_health_alerts` | Comprehensive risk warnings |
| `get_whale_activity` | Large transfer monitoring |
| `construct_revoke_approval` | Build revoke transaction data |

### 🌐 Network & Contracts
| Tool | Description |
|------|-------------|
| `get_gas_price` | Current gas + operation cost estimates |
| `get_block_info` | Block details by number or 'latest' |
| `get_cro_overview` | CRO price, market cap, network status |
| `get_protocol_stats` | Protocol TVL summaries |
| `search_contract` | Search by name, symbol, or address |
| `get_contract_info` | Contract type & code details |
| `resolve_cronos_id` | .cro domain ↔ address resolution |
| `construct_swap_tx` | Build swap calldata (approval-aware) |

---

## 🚀 Quick Start

### For MCP Clients (Claude, etc.)

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
        "address": "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23"
      }
    }
  }'
```

---

## 🏗️ Architecture

<div align="center">

```
┌──────────────────────────────────────────────────────────────────────┐
│                         USER LAYER                                    │
├────────────┬────────────┬────────────┬──────────────────────────────┤
│ MCP Client │  AI Agent  │  Web UI    │  Direct API                  │
│            │  (Custom)  │ (Playground)│  (curl/SDK)                 │
└─────┬──────┴─────┬──────┴─────┬──────┴──────────┬────────────────────┘
      │            │            │                 │
      └────────────┴────────────┴─────────────────┘
                          │
                 JSON-RPC 2.0 (HTTP)
                          │
                          ▼
┌──────────────────────────────────────────────────────────────────────┐
│                    CROLENS API (Cloudflare Workers)                  │
├──────────────────────────────────────────────────────────────────────┤
│  🔐 Gateway: Auth (API Key) + Rate Limit + x402 Payment             │
├──────────────────────────────────────────────────────────────────────┤
│  📡 MCP Layer: tools/list + tools/call                              │
├──────────────────────────────────────────────────────────────────────┤
│  🧠 Domain: 30 Semantic Tools                                        │
├──────────────────────────────────────────────────────────────────────┤
│  🔌 Adapters: VVS (UniswapV2) + Tectonic (CompoundV2)               │
├──────────────────────────────────────────────────────────────────────┤
│  ⚙️ Infra: RPC + Multicall3 + KV Cache + D1 Database                │
└──────────────────────────────────────────────────────────────────────┘
                          │
                          ▼
                   ┌──────────────┐
                   │ CRONOS CHAIN │
                   └──────────────┘
```

</div>

---

## 🛠️ Tech Stack

<div align="center">

| Layer | Technology |
|:-----:|:----------:|
| **Backend** | ![Rust](https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white) ![Cloudflare](https://img.shields.io/badge/Workers-F38020?style=flat-square&logo=cloudflare&logoColor=white) ![alloy](https://img.shields.io/badge/alloy--rs-3C3C3D?style=flat-square) |
| **Frontend** | ![React](https://img.shields.io/badge/React-61DAFB?style=flat-square&logo=react&logoColor=black) ![Vite](https://img.shields.io/badge/Vite-646CFF?style=flat-square&logo=vite&logoColor=white) ![TailwindCSS](https://img.shields.io/badge/Tailwind-06B6D4?style=flat-square&logo=tailwindcss&logoColor=white) |
| **Storage** | ![Cloudflare KV](https://img.shields.io/badge/KV-F38020?style=flat-square&logo=cloudflare&logoColor=white) ![D1](https://img.shields.io/badge/D1-F38020?style=flat-square&logo=cloudflare&logoColor=white) |
| **Web3** | ![RainbowKit](https://img.shields.io/badge/RainbowKit-7B3FE4?style=flat-square) ![wagmi](https://img.shields.io/badge/wagmi-1C1C1C?style=flat-square) |

</div>

---

## 📁 Project Structure

```
crolens/
├── crolens-api/              # Rust backend (Cloudflare Worker)
│   ├── src/
│   │   ├── lib.rs            # Entry + Cron handler
│   │   ├── gateway/          # Auth + x402 payment
│   │   ├── mcp/              # MCP protocol (router, tools)
│   │   ├── domain/           # 30 tool implementations
│   │   ├── adapters/         # VVS + Tectonic adapters
│   │   └── infra/            # RPC, multicall, cache
│   └── wrangler.toml
│
└── crolens-web/              # React frontend
    └── src/
        ├── features/         # Playground, Dashboard, Console
        ├── components/       # P5-style UI components
        └── lib/              # API client, wagmi config
```

---

## 🎮 Live Demo

<div align="center">

| Page | Description | Link |
|:----:|:-----------:|:----:|
| **Playground** | Interactive tool testing | [![Playground](https://img.shields.io/badge/TRY-Playground-D90018?style=for-the-badge)](https://crolens-web.pages.dev/playground) |
| **Dashboard** | Real-time monitoring | [![Dashboard](https://img.shields.io/badge/VIEW-Dashboard-333?style=for-the-badge)](https://crolens-web.pages.dev/dashboard) |
| **Console** | API key management | [![Console](https://img.shields.io/badge/MANAGE-Console-333?style=for-the-badge)](https://crolens-web.pages.dev/console) |

</div>

---

## 📊 Performance

| Tool | Latency | Description |
|------|:-------:|-------------|
| `search_contract` | ~50ms | D1 query only |
| `simulate_transaction` | ~270ms | Single RPC |
| `get_defi_positions` | ~290ms | Batch RPC |
| `decode_transaction` | ~450ms | 2x RPC |
| `get_account_summary` | ~550ms | Multi RPC + pricing |
| `construct_swap_tx` | ~900ms | Multi RPC + simulation |

---

## 🔗 Contract Addresses (Cronos Mainnet)

| Contract | Address |
|----------|---------|
| Multicall3 | `0xcA11bde05977b3631167028862bE2a173976CA11` |
| WCRO | `0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23` |
| USDC | `0xc21223249CA28397B4B6541dfFaEcC539BfF0c59` |
| VVS Router | `0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae` |
| VVS Factory | `0x3B44B2a187a7b3824131F8db5a74194D0a42Fc15` |
| TectonicCore | `0x7De56Bd8b37827c51835e162c867848fE2403a48` |

---

## 🧑‍💻 Development

### Prerequisites

- Rust 1.75+ with `wasm32-unknown-unknown` target
- Node.js 18+
- Wrangler CLI

### Backend

```bash
cd crolens-api
cp .dev.vars.example .dev.vars
wrangler d1 execute crolens-db --local --file=./db/schema.sql
wrangler d1 execute crolens-db --local --file=./db/seed.sql
wrangler dev --local --persist-to .wrangler/state
```

### Frontend

```bash
cd crolens-web
npm install
npm run dev
```

---

## 📝 License

MIT

---

<div align="center">

**Built with ❤️ for [Cronos Hackathon 2025](https://cronos.org/)**

[![GitHub](https://img.shields.io/badge/GitHub-strawcatcher/CroLens-181717?style=for-the-badge&logo=github)](https://github.com/strawcatcher/CroLens)

</div>

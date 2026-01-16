export type ToolName =
  | "get_account_summary"
  | "get_defi_positions"
  | "decode_transaction"
  | "simulate_transaction"
  | "search_contract"
  | "construct_swap_tx"
  | "get_token_info"
  | "get_pool_info"
  | "get_gas_price"
  | "get_token_price"
  | "get_approval_status"
  | "get_block_info";

export type JsonRpcId = number | string | null;

export interface JsonRpcError<Data = unknown> {
  code: number;
  message: string;
  data?: Data;
}

export interface JsonRpcResponse<Result = unknown, Data = unknown> {
  jsonrpc: "2.0";
  id: JsonRpcId;
  result?: Result;
  error?: JsonRpcError<Data>;
}

export interface Meta {
  trace_id?: string;
  timestamp: number;
  latency_ms: number;
  cached?: boolean;
}

export interface SimpleTextResult {
  text: string;
  meta: Meta;
}

export interface WalletToken {
  token_address: string;
  symbol: string;
  decimals: number;
  balance: string;
  balance_formatted: string;
  price_usd: string | null;
  value_usd: string | null;
}

export interface AccountSummary {
  address: string;
  total_net_worth_usd: string;
  wallet: WalletToken[];
  defi_summary: {
    total_defi_value_usd: string;
    vvs_liquidity_usd: string;
    tectonic_supply_usd: string;
    tectonic_borrow_usd: string;
  };
  meta: Meta;
}

export type AccountSummaryResult = AccountSummary | SimpleTextResult;

export interface DefiTokenAmount {
  address: string;
  symbol: string;
  amount: string;
  amount_formatted: string;
}

export interface VvsPosition {
  pool_id: string;
  pool_name: string;
  lp_amount: string;
  lp_wallet_amount?: string;
  lp_staked_amount?: string;
  token0: DefiTokenAmount;
  token1: DefiTokenAmount;
  liquidity_usd?: string | null;
  pending_vvs: string;
  pending_vvs_formatted: string;
  pending_rewards?: { vvs: string };
  pending_rewards_usd?: string | null;
  apy: unknown;
}

export interface TectonicSupply {
  market_address: string;
  asset_symbol: string;
  supply_balance: string;
  supply_balance_usd: string | null;
  supply_apy: unknown;
  is_collateral: boolean;
}

export interface TectonicBorrow {
  market_address: string;
  asset_symbol: string;
  borrow_balance: string;
  borrow_balance_usd: string | null;
  borrow_apy: unknown;
}

export interface DefiPositions {
  address: string;
  vvs: {
    total_liquidity_usd: string;
    total_pending_rewards_usd: string;
    positions: VvsPosition[];
  };
  tectonic: {
    total_supply_usd: string;
    total_borrow_usd: string;
    net_value_usd: string;
    health_factor: string;
    supplies: TectonicSupply[];
    borrows: TectonicBorrow[];
  };
  meta: Meta;
}

export type DefiPositionsResult = DefiPositions | SimpleTextResult;

export interface DecodedTransaction {
  hash: string;
  from: string;
  to: string;
  action: string;
  protocol: string | null;
  status: string;
  gas_used: string;
  decoded: {
    method_name: string;
    params: unknown;
  };
  meta: Meta;
}

export type DecodedTransactionResult = DecodedTransaction | SimpleTextResult;

export interface SimulationStateChange {
  type: string;
  description: string;
  from: string;
  to: string;
  amount: string;
  token: string;
}

export interface SimulationResult {
  success: boolean;
  simulation_available?: boolean;
  gas_estimated?: string;
  estimated_cost_cro?: string | null;
  return_data?: string | null;
  decoded_return?: unknown;
  state_changes?: SimulationStateChange[];
  risk_assessment?: {
    level: string;
    warnings: string[];
  };
  meta: Meta;
}

export type SimulationResultOrText = SimulationResult | SimpleTextResult;

export interface ContractSearchResult {
  name: string | null;
  address: string | null;
  type: string | null;
  protocol: string | null;
}

export interface ContractSearchResponse {
  results: ContractSearchResult[];
  meta: Meta;
}

export interface SwapPipelineStep {
  step_index: number;
  type: "approval" | "swap" | string;
  description: string;
  tx_data: {
    to: string;
    data: string;
    value: string;
  };
  status: "pending" | "blocked" | string;
}

export interface SwapPipeline {
  operation_id: string;
  estimated_out: string;
  minimum_out: string;
  price_impact: string;
  simulation_verified: boolean;
  steps: SwapPipelineStep[];
  meta: Meta;
}

export interface HealthResponse {
  status: "ok" | "degraded" | "unhealthy";
  version: string;
  checks: {
    db: { status: "ok" | "error"; latency_ms: number; error?: string | null };
    kv: { status: "ok" | "error"; latency_ms: number; error?: string | null };
    rpc: { status: "ok" | "error"; latency_ms: number; error?: string | null };
  };
  timestamp: number;
}

export interface StatsResponse {
  protocols_supported: number;
  meta: Meta;
}

export interface HttpErrorBody {
  message: string;
}

export interface HttpErrorResponse {
  error: HttpErrorBody;
  meta?: Meta;
  status?: string;
}

export interface X402QuoteResponse {
  chain_id: number;
  payment_address: string;
  credits: number;
  amount_wei: string;
  price_per_credit_wei: string;
  meta: Meta;
}

export interface X402StatusResponse {
  api_key: string;
  tier: string;
  credits: number;
  meta: Meta;
}

export interface X402VerifyResponse {
  status: "pending" | "credited" | "already_credited" | "failed" | "rejected";
  tx_hash: string;
  credits_added?: number;
  credits?: number;
  tier?: string;
  meta: Meta;
  error?: HttpErrorBody;
}

// New tool types

export interface TokenPool {
  dex: string;
  pair: string;
  pair_address: string;
  tvl_usd: string;
}

export interface TokenInfoResult {
  address: string;
  name: string;
  symbol: string;
  decimals: number;
  total_supply: string;
  price_usd: string | null;
  market_cap_usd: string | null;
  pools: TokenPool[];
  meta: Meta;
}

export interface PoolInfoResult {
  pool_address: string;
  dex: string;
  token0: {
    address: string;
    symbol: string;
    reserve: string;
  };
  token1: {
    address: string;
    symbol: string;
    reserve: string;
  };
  total_supply: string;
  tvl_usd: string | null;
  apy: string | null;
  meta: Meta;
}

export interface GasEstimate {
  operation: string;
  gas_units: number;
  cost_cro: string;
  cost_usd: string | null;
}

export interface GasPriceResult {
  gas_price_gwei: string;
  gas_price_wei: string;
  level: "low" | "medium" | "high";
  estimates: GasEstimate[];
  recommendation: string;
  meta: Meta;
}

export interface TokenPriceEntry {
  token: string;
  address: string | null;
  price_usd: string | null;
  source: string;
  confidence: string;
}

export interface TokenPriceResult {
  prices: TokenPriceEntry[];
  meta: Meta;
}

// Approval Status Types

export interface ApprovalEntry {
  token_symbol: string;
  token_address: string;
  spender_address: string;
  spender_name: string;
  protocol: string;
  allowance: string;
  is_unlimited: boolean;
  risk_level: "safe" | "warning" | "danger";
}

export interface ApprovalStatusResult {
  address: string;
  approvals: ApprovalEntry[];
  summary: {
    total_approvals: number;
    unlimited_approvals: number;
    risk_score: number;
  };
  meta: Meta;
}

// Block Info Types

export interface BlockInfoResult {
  number: number;
  hash: string;
  timestamp: number;
  timestamp_relative: string;
  transactions_count: number;
  gas_used: string;
  gas_limit: string;
  gas_used_percent: string;
  base_fee_gwei: string;
  miner: string;
  meta: Meta;
}

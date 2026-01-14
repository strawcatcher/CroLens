import type {
  HealthResponse,
  HttpErrorResponse,
  JsonRpcError,
  JsonRpcResponse,
  StatsResponse,
  ToolName,
  X402QuoteResponse,
  X402StatusResponse,
  X402VerifyResponse,
} from "@/types/api";
import { reportApiError } from "@/lib/monitoring";

export const DEFAULT_TIMEOUT_MS = 30_000;

export class McpClientError extends Error {
  readonly code?: number;
  readonly data?: unknown;
  readonly httpStatus?: number;
  readonly traceId?: string;

  constructor(
    message: string,
    opts?: {
      code?: number;
      data?: unknown;
      httpStatus?: number;
      traceId?: string;
    },
  ) {
    super(message);
    this.name = "McpClientError";
    this.code = opts?.code;
    this.data = opts?.data;
    this.httpStatus = opts?.httpStatus;
    this.traceId = opts?.traceId;
  }
}

export class HttpApiError extends Error {
  readonly httpStatus: number;
  readonly body?: unknown;

  constructor(message: string, opts: { httpStatus: number; body?: unknown }) {
    super(message);
    this.name = "HttpApiError";
    this.httpStatus = opts.httpStatus;
    this.body = opts.body;
  }
}

export interface McpCallOptions {
  apiUrl?: string;
  apiKey?: string;
  traceId?: string;
  timeoutMs?: number;
  signal?: AbortSignal;
}

export interface McpCallResult<Result = unknown> {
  traceId: string;
  durationMs: number;
  response: JsonRpcResponse<Result>;
}

let nextJsonRpcId = 1;

export function getDefaultApiUrl() {
  const raw =
    typeof import.meta.env.VITE_API_URL === "string"
      ? import.meta.env.VITE_API_URL.trim()
      : "";
  return raw.length > 0 ? raw : "http://localhost:8787";
}

export function createTraceId() {
  return (
    globalThis.crypto?.randomUUID?.() ??
    `${Date.now()}-${Math.random().toString(16).slice(2)}`
  );
}

function normalizeApiUrl(url: string) {
  return url.trim().replace(/\/+$/, "");
}

function parseJsonRpcError(value: unknown): JsonRpcError | null {
  if (!value || typeof value !== "object") return null;
  const error = (value as { error?: unknown }).error;
  if (!error || typeof error !== "object") return null;
  const code = (error as { code?: unknown }).code;
  const message = (error as { message?: unknown }).message;
  if (typeof code !== "number" || typeof message !== "string") return null;
  const data = (error as { data?: unknown }).data;
  return { code, message, data };
}

async function fetchJson<T>(
  url: string,
  init?: RequestInit,
): Promise<{ data: T; status: number }> {
  let res: Response;
  try {
    res = await fetch(url, init);
  } catch (err) {
    reportApiError(err);
    throw err;
  }
  const status = res.status;

  let payload: unknown;
  try {
    payload = await res.json();
  } catch (err) {
    const apiError = new HttpApiError("Invalid JSON response", {
      httpStatus: status,
      body: String(err),
    });
    reportApiError(apiError);
    throw apiError;
  }

  if (!res.ok) {
    const maybeError = payload as Partial<HttpErrorResponse>;
    const message =
      typeof maybeError?.error?.message === "string"
        ? maybeError.error.message
        : `HTTP ${status}`;
    const apiError = new HttpApiError(message, { httpStatus: status, body: payload });
    reportApiError(apiError);
    throw apiError;
  }

  return { data: payload as T, status };
}

export async function mcpCallRaw<Result>(
  tool: ToolName,
  args: Record<string, unknown>,
  options: McpCallOptions = {},
): Promise<McpCallResult<Result>> {
  const apiUrl = normalizeApiUrl(options.apiUrl ?? getDefaultApiUrl());
  const apiKey = options.apiKey?.trim() ?? "";
  const traceId = options.traceId ?? createTraceId();
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;

  const controller = new AbortController();
  const onAbort = () => controller.abort();
  options.signal?.addEventListener("abort", onAbort);

  const timeoutHandle = setTimeout(() => controller.abort(), timeoutMs);
  const started = Date.now();

  try {
    const id = nextJsonRpcId++;
    const body = {
      jsonrpc: "2.0",
      id,
      method: "tools/call",
      params: {
        name: tool,
        arguments: args,
      },
    };

    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "x-request-id": traceId,
    };
    if (apiKey.length > 0) headers["x-api-key"] = apiKey;

    const res = await fetch(`${apiUrl}/`, {
      method: "POST",
      headers,
      body: JSON.stringify(body),
      signal: controller.signal,
    });

    const durationMs = Date.now() - started;

    let payload: unknown;
    try {
      payload = await res.json();
    } catch (err) {
      const apiError = new McpClientError("Invalid JSON response", {
        httpStatus: res.status,
        traceId,
        data: String(err),
      });
      reportApiError(apiError);
      throw apiError;
    }

    const json = payload as JsonRpcResponse<Result>;
    const rpcError = parseJsonRpcError(json);
    if (rpcError) {
      const apiError = new McpClientError(rpcError.message, {
        code: rpcError.code,
        data: rpcError.data,
        httpStatus: res.status,
        traceId,
      });
      reportApiError(apiError);
      throw apiError;
    }

    if (!res.ok) {
      const apiError = new McpClientError(`HTTP ${res.status}`, {
        httpStatus: res.status,
        traceId,
        data: json,
      });
      reportApiError(apiError);
      throw apiError;
    }

    return { traceId, durationMs, response: json };
  } catch (err) {
    if (err instanceof McpClientError) throw err;
    if (controller.signal.aborted) {
      const apiError = new McpClientError("Request aborted", { traceId });
      reportApiError(apiError);
      throw apiError;
    }
    const apiError = new McpClientError(String(err), { traceId });
    reportApiError(apiError);
    throw apiError;
  } finally {
    clearTimeout(timeoutHandle);
    options.signal?.removeEventListener("abort", onAbort);
  }
}

export async function mcpCall<Result>(
  tool: ToolName,
  args: Record<string, unknown>,
  options: McpCallOptions = {},
): Promise<Result> {
  const { response } = await mcpCallRaw<Result>(tool, args, options);
  if (!("result" in response)) {
    const apiError = new McpClientError("Missing JSON-RPC result", {
      traceId: options.traceId,
    });
    reportApiError(apiError);
    throw apiError;
  }
  return response.result as Result;
}

export async function fetchHealth(apiUrl?: string): Promise<HealthResponse> {
  const base = normalizeApiUrl(apiUrl ?? getDefaultApiUrl());
  const { data } = await fetchJson<HealthResponse>(`${base}/health`, {
    method: "GET",
  });
  return data;
}

export async function fetchStats(apiUrl?: string): Promise<StatsResponse> {
  const base = normalizeApiUrl(apiUrl ?? getDefaultApiUrl());
  const { data } = await fetchJson<StatsResponse>(`${base}/stats`, {
    method: "GET",
  });
  return data;
}

export async function fetchX402Quote(apiUrl?: string): Promise<X402QuoteResponse> {
  const base = normalizeApiUrl(apiUrl ?? getDefaultApiUrl());
  const { data } = await fetchJson<X402QuoteResponse>(`${base}/x402/quote`, {
    method: "GET",
  });
  return data;
}

export async function fetchX402Status(options: McpCallOptions = {}): Promise<X402StatusResponse> {
  const base = normalizeApiUrl(options.apiUrl ?? getDefaultApiUrl());
  const apiKey = options.apiKey?.trim() ?? "";
  const traceId = options.traceId ?? createTraceId();
  if (apiKey.length === 0) throw new Error("Missing apiKey");

  const { data } = await fetchJson<X402StatusResponse>(`${base}/x402/status`, {
    method: "GET",
    headers: {
      "x-api-key": apiKey,
      "x-request-id": traceId,
    },
  });
  return data;
}

export async function verifyX402Payment(
  txHash: string,
  options: McpCallOptions = {},
): Promise<X402VerifyResponse> {
  const base = normalizeApiUrl(options.apiUrl ?? getDefaultApiUrl());
  const apiKey = options.apiKey?.trim() ?? "";
  const traceId = options.traceId ?? createTraceId();
  if (apiKey.length === 0) throw new Error("Missing apiKey");

  const { data } = await fetchJson<X402VerifyResponse>(`${base}/x402/verify`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": apiKey,
      "x-request-id": traceId,
    },
    body: JSON.stringify({ tx_hash: txHash }),
  });
  return data;
}

export function getAccountSummary(
  args: { address: string; simpleMode?: boolean },
  options?: McpCallOptions,
) {
  return mcpCall(
    "get_account_summary",
    { address: args.address, simple_mode: !!args.simpleMode },
    options,
  );
}

export function getDefiPositions(
  args: { address: string; simpleMode?: boolean },
  options?: McpCallOptions,
) {
  return mcpCall(
    "get_defi_positions",
    { address: args.address, simple_mode: !!args.simpleMode },
    options,
  );
}

export function decodeTransaction(
  args: { txHash: string; simpleMode?: boolean },
  options?: McpCallOptions,
) {
  return mcpCall(
    "decode_transaction",
    { tx_hash: args.txHash, simple_mode: !!args.simpleMode },
    options,
  );
}

export function simulateTransaction(
  args: {
    from: string;
    to: string;
    data: string;
    value: string;
    simpleMode?: boolean;
  },
  options?: McpCallOptions,
) {
  return mcpCall(
    "simulate_transaction",
    {
      from: args.from,
      to: args.to,
      data: args.data,
      value: args.value,
      simple_mode: !!args.simpleMode,
    },
    options,
  );
}

export function searchContract(
  args: { query: string; limit?: number },
  options?: McpCallOptions,
) {
  return mcpCall(
    "search_contract",
    { query: args.query, limit: args.limit },
    options,
  );
}

export function constructSwapTx(
  args: {
    from: string;
    tokenIn: string;
    tokenOut: string;
    amountIn: string;
    slippageBps: number;
  },
  options?: McpCallOptions,
) {
  return mcpCall(
    "construct_swap_tx",
    {
      from: args.from,
      token_in: args.tokenIn,
      token_out: args.tokenOut,
      amount_in: args.amountIn,
      slippage_bps: args.slippageBps,
    },
    options,
  );
}

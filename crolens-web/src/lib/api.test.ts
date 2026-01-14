import { describe, expect, it, vi, afterEach } from "vitest";
import {
  fetchHealth,
  fetchX402Quote,
  fetchX402Status,
  mcpCall,
  mcpCallRaw,
  McpClientError,
  verifyX402Payment,
  getDefaultApiUrl,
} from "@/lib/api";

function jsonResponse(payload: unknown, init?: ResponseInit) {
  return new Response(JSON.stringify(payload), {
    status: 200,
    headers: { "Content-Type": "application/json" },
    ...init,
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("getDefaultApiUrl", () => {
  it("falls back to localhost by default", () => {
    expect(getDefaultApiUrl()).toBe("http://localhost:8787");
  });
});

describe("mcpCallRaw", () => {
  it("sends JSON-RPC payload with headers", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse({ jsonrpc: "2.0", id: 1, result: { ok: true } }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await mcpCallRaw("get_account_summary", { address: "0xabc" }, {
      apiUrl: "https://example.com/",
      apiKey: "k1",
      traceId: "trace-req",
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("https://example.com/");
    expect(init.method).toBe("POST");
    expect(init.headers).toMatchObject({
      "Content-Type": "application/json",
      "x-api-key": "k1",
      "x-request-id": "trace-req",
    });

    const body = JSON.parse(String(init.body));
    expect(body.method).toBe("tools/call");
    expect(body.params.name).toBe("get_account_summary");
    expect(body.params.arguments.address).toBe("0xabc");
  });

  it("throws McpClientError for JSON-RPC errors", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(
        jsonResponse({
          jsonrpc: "2.0",
          id: 1,
          error: { code: -32002, message: "Payment required" },
        }),
      );
    vi.stubGlobal("fetch", fetchMock);

    await expect(
      mcpCallRaw("get_account_summary", { address: "0xabc" }, { traceId: "t1" }),
    ).rejects.toMatchObject({
      name: "McpClientError",
      code: -32002,
      traceId: "t1",
    });
  });

  it("throws McpClientError for non-OK HTTP responses", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(
        jsonResponse({ jsonrpc: "2.0", id: 1, result: { ok: true } }, { status: 500 }),
      );
    vi.stubGlobal("fetch", fetchMock);

    await expect(
      mcpCallRaw("get_account_summary", { address: "0xabc" }, { traceId: "t1" }),
    ).rejects.toMatchObject({
      name: "McpClientError",
      httpStatus: 500,
    });
  });
});

describe("mcpCall", () => {
  it("throws when JSON-RPC result is missing", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(jsonResponse({ jsonrpc: "2.0", id: 1 }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(
      mcpCall("get_account_summary", { address: "0xabc" }, { traceId: "t1" }),
    ).rejects.toBeInstanceOf(McpClientError);
  });
});

describe("fetchHealth", () => {
  it("fetches /health", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(jsonResponse({ status: "ok", version: "0.1.0", timestamp: 1 }));
    vi.stubGlobal("fetch", fetchMock);

    const resp = await fetchHealth("https://api.example.com/");
    expect(resp.status).toBe("ok");
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.example.com/health",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("throws for non-OK health responses", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse({ error: { message: "down" } }, { status: 503 }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await expect(fetchHealth("https://api.example.com")).rejects.toMatchObject({
      name: "HttpApiError",
      httpStatus: 503,
    });
  });
});

describe("x402 helpers", () => {
  it("fetches /x402/quote", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse({
        chain_id: 25,
        payment_address: "0xabc",
        credits: 1000,
        amount_wei: "1",
        price_per_credit_wei: "1",
        meta: { timestamp: 1, latency_ms: 1 },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await fetchX402Quote("https://api.example.com/");
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.example.com/x402/quote",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("requires apiKey for /x402/status", async () => {
    await expect(fetchX402Status({ apiUrl: "https://api.example.com", apiKey: "" })).rejects.toThrow(
      "Missing apiKey",
    );
  });

  it("posts to /x402/verify with tx_hash", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse({
        status: "pending",
        tx_hash: "0xdeadbeef",
        meta: { timestamp: 1, latency_ms: 1 },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await verifyX402Payment("0xdeadbeef", {
      apiUrl: "https://api.example.com/",
      apiKey: "k1",
      traceId: "trace-verify",
    });

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("https://api.example.com/x402/verify");
    expect(init.method).toBe("POST");
    expect(init.headers).toMatchObject({
      "Content-Type": "application/json",
      "x-api-key": "k1",
      "x-request-id": "trace-verify",
    });
    expect(JSON.parse(String(init.body))).toEqual({ tx_hash: "0xdeadbeef" });
  });
});


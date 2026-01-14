import { describe, expect, it } from "vitest";
import { McpClientError } from "@/lib/api";
import { getMcpErrorMessage } from "@/lib/errors";

describe("getMcpErrorMessage", () => {
  it("maps known MCP error codes", () => {
    const err = new McpClientError("Payment required", {
      code: -32002,
      traceId: "trace-1",
    });

    expect(getMcpErrorMessage(err)).toEqual({
      message: "Payment required: insufficient credits (top up via x402).",
      code: -32002,
      traceId: "trace-1",
    });
  });

  it("falls back to original message for unknown codes", () => {
    const err = new McpClientError("Boom", { code: 123 });
    expect(getMcpErrorMessage(err).message).toBe("Boom");
  });

  it("returns message for plain Error", () => {
    expect(getMcpErrorMessage(new Error("oops"))).toEqual({ message: "oops" });
  });

  it("stringifies non-Error values", () => {
    expect(getMcpErrorMessage("bad")).toEqual({ message: "bad" });
  });

  it("keeps traceId when present", () => {
    const err = new McpClientError("Rate limit exceeded", {
      code: -32003,
      traceId: "trace-2",
    });
    const out = getMcpErrorMessage(err);
    expect(out.code).toBe(-32003);
    expect(out.traceId).toBe("trace-2");
  });

  it("maps invalid params code", () => {
    const err = new McpClientError("Invalid params", { code: -32602 });
    expect(getMcpErrorMessage(err).message).toContain("Invalid params");
  });

  it("maps fetch network errors to a friendly message", () => {
    const err = new TypeError("Failed to fetch");
    expect(getMcpErrorMessage(err)).toEqual({
      message: "Network error. Please retry.",
    });
  });

  it("maps network-like McpClientError without code", () => {
    const err = new McpClientError("TypeError: Failed to fetch", {
      traceId: "trace-net",
    });
    expect(getMcpErrorMessage(err)).toEqual({
      message: "Network error. Please retry.",
      traceId: "trace-net",
    });
  });
});

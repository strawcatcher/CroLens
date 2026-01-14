import { McpClientError } from "@/lib/api";

const MCP_ERROR_MESSAGES: Record<number, string> = {
  [-32002]: "Payment required: insufficient credits (top up via x402).",
  [-32003]: "Rate limit exceeded. Please retry later.",
  [-32500]: "RPC error. Please retry.",
  [-32501]: "Service unavailable. Please retry later.",
  [-32602]: "Invalid params. Please check your input.",
  [-32601]: "Tool not found.",
  [-32600]: "Invalid request.",
};

function isLikelyNetworkError(err: Error) {
  const msg = err.message ?? "";
  return isLikelyNetworkErrorMessage(msg) || (err.name === "TypeError" && /fetch/i.test(msg));
}

function isLikelyNetworkErrorMessage(message: string) {
  return (
    /failed to fetch/i.test(message) ||
    /network\s?error/i.test(message) ||
    /net::err/i.test(message)
  );
}

export function getMcpErrorMessage(err: unknown): {
  message: string;
  code?: number;
  traceId?: string;
} {
  if (err instanceof McpClientError) {
    if (typeof err.code !== "number" && isLikelyNetworkErrorMessage(err.message)) {
      return { message: "Network error. Please retry.", traceId: err.traceId };
    }
    const message =
      (typeof err.code === "number" ? MCP_ERROR_MESSAGES[err.code] : null) ??
      err.message;
    return { message, code: err.code, traceId: err.traceId };
  }

  if (err instanceof Error) {
    if (isLikelyNetworkError(err)) {
      return { message: "Network error. Please retry." };
    }
    return { message: err.message };
  }

  return { message: String(err) };
}

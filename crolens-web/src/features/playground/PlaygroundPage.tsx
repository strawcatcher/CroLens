import * as React from "react";
import { useAccount } from "wagmi";
import { toast } from "sonner";
import { Eraser, Link2, Play, ClipboardPaste, Activity, Box, LayoutDashboard } from "lucide-react";
import { P5Title, P5Card, P5Button, P5Input } from "@/components/p5";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { CodeBlock } from "@/components/ui/code-block";
import { mcpCallRaw } from "@/lib/api";
import { getMcpErrorMessage } from "@/lib/errors";
import { useAppStore } from "@/stores/app";
import { ToolSelector } from "@/features/playground/ToolSelector";
import type {
  AccountSummary,
  ContractSearchResponse,
  DecodedTransaction,
  DefiPositions,
  JsonRpcResponse,
  Meta,
  SimulationResult,
  SwapPipeline,
  ToolName,
} from "@/types/api";

type ToolResult =
  | AccountSummary
  | DefiPositions
  | DecodedTransaction
  | SimulationResult
  | ContractSearchResponse
  | SwapPipeline
  | { text: string; meta: Meta };

const TOOL_OPTIONS: Array<{
  value: ToolName;
  label: string;
  description: string;
}> = [
  {
    value: "get_account_summary",
    label: "GET_ACCOUNT_SUMMARY",
    description: "Wallet balances + DeFi summary",
  },
  {
    value: "get_defi_positions",
    label: "GET_DEFI_POSITIONS",
    description: "VVS + Tectonic positions",
  },
  {
    value: "decode_transaction",
    label: "DECODE_TRANSACTION",
    description: "Decode tx hash to action",
  },
  {
    value: "simulate_transaction",
    label: "SIMULATE_TRANSACTION",
    description: "Simulate tx execution",
  },
  {
    value: "search_contract",
    label: "SEARCH_CONTRACT",
    description: "Search contracts by name",
  },
  {
    value: "construct_swap_tx",
    label: "CONSTRUCT_SWAP_TX",
    description: "Build swap pipeline",
  },
];

function isAddress(value: string) {
  return /^0x[a-fA-F0-9]{40}$/.test(value.trim());
}

function isTxHash(value: string) {
  return /^0x[a-fA-F0-9]{64}$/.test(value.trim());
}

function isHexData(value: string) {
  const trimmed = value.trim();
  if (!trimmed.startsWith("0x")) return false;
  return /^[a-fA-F0-9]*$/.test(trimmed.slice(2));
}

function extractMeta(value: unknown): Meta | null {
  if (!value || typeof value !== "object") return null;
  const meta = (value as { meta?: unknown }).meta;
  if (!meta || typeof meta !== "object") return null;
  const timestamp = (meta as { timestamp?: unknown }).timestamp;
  const latencyMs = (meta as { latency_ms?: unknown }).latency_ms;
  if (typeof timestamp !== "number" || typeof latencyMs !== "number")
    return null;
  const traceId = (meta as { trace_id?: unknown }).trace_id;
  return {
    timestamp,
    latency_ms: latencyMs,
    cached: (meta as { cached?: unknown }).cached as boolean | undefined,
    trace_id: typeof traceId === "string" ? traceId : undefined,
  };
}

function renderMeta(meta: Meta | null) {
  if (!meta) return null;
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs text-[#A3A3A3] font-mono">
      <span>latency: {meta.latency_ms}ms</span>
      {typeof meta.trace_id === "string" ? (
        <span>trace: {meta.trace_id}</span>
      ) : null}
    </div>
  );
}

export function PlaygroundPage() {
  const apiKey = useAppStore((s) => s.apiKey);
  const addLog = useAppStore((s) => s.addLog);
  const addLatency = useAppStore((s) => s.addLatency);
  const { address: connectedAddress, isConnected } = useAccount();

  const baseId = React.useId();
  const addressInputId = `${baseId}-address`;
  const txHashInputId = `${baseId}-txhash`;
  const simulateToInputId = `${baseId}-simulate-to`;
  const simulateDataInputId = `${baseId}-simulate-data`;
  const simulateValueInputId = `${baseId}-simulate-value`;
  const searchQueryInputId = `${baseId}-search-query`;
  const searchLimitInputId = `${baseId}-search-limit`;
  const swapTokenInInputId = `${baseId}-swap-token-in`;
  const swapTokenOutInputId = `${baseId}-swap-token-out`;
  const swapAmountInInputId = `${baseId}-swap-amount-in`;
  const swapSlippageInputId = `${baseId}-swap-slippage`;
  const simpleModeSwitchId = `${baseId}-simple-mode`;

  const [tool, setTool] = React.useState<ToolName>("get_account_summary");

  const [address, setAddress] = React.useState("");
  const [simpleMode, setSimpleMode] = React.useState(false);

  const [txHash, setTxHash] = React.useState("");
  const [simulateTo, setSimulateTo] = React.useState("");
  const [simulateData, setSimulateData] = React.useState("0x");
  const [simulateValue, setSimulateValue] = React.useState("0");

  const [searchQuery, setSearchQuery] = React.useState("");
  const [searchLimit, setSearchLimit] = React.useState(20);

  const [swapTokenIn, setSwapTokenIn] = React.useState("CRO");
  const [swapTokenOut, setSwapTokenOut] = React.useState("USDC");
  const [swapAmountIn, setSwapAmountIn] = React.useState("1000000000000000000");
  const [swapSlippageBps, setSwapSlippageBps] = React.useState(50);

  const [showRaw, setShowRaw] = React.useState(false);
  const [, setExecutionLog] = React.useState<string[]>([]);
  const [rawResponse, setRawResponse] =
    React.useState<JsonRpcResponse<ToolResult> | null>(null);
  const [result, setResult] = React.useState<ToolResult | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(false);

  const supportsSimpleMode =
    tool === "get_account_summary" ||
    tool === "get_defi_positions" ||
    tool === "decode_transaction" ||
    tool === "simulate_transaction";

  const needsAddress =
    tool === "get_account_summary" ||
    tool === "get_defi_positions" ||
    tool === "simulate_transaction" ||
    tool === "construct_swap_tx";

  const toolHelp =
    TOOL_OPTIONS.find((t) => t.value === tool)?.description ?? "";

  const canExecute = React.useMemo(() => {
    if (needsAddress && !isAddress(address)) return false;
    if (tool === "decode_transaction" && !isTxHash(txHash)) return false;
    if (
      tool === "simulate_transaction" &&
      (!isAddress(simulateTo) || !isHexData(simulateData))
    )
      return false;
    if (tool === "search_contract" && searchQuery.trim().length === 0)
      return false;
    if (tool === "construct_swap_tx") {
      if (swapTokenIn.trim().length === 0) return false;
      if (swapTokenOut.trim().length === 0) return false;
      if (!/^\d+$/.test(swapAmountIn.trim())) return false;
      if (swapSlippageBps < 0 || swapSlippageBps > 5000) return false;
    }
    return true;
  }, [
    address,
    needsAddress,
    searchQuery,
    simulateData,
    simulateTo,
    swapAmountIn,
    swapSlippageBps,
    swapTokenIn,
    swapTokenOut,
    tool,
    txHash,
  ]);

  async function onPaste() {
    try {
      const text = await navigator.clipboard.readText();
      setAddress(text.trim());
      toast.success("Pasted");
    } catch (err) {
      toast.error(String(err));
    }
  }

  function useConnected() {
    if (!connectedAddress) return;
    setAddress(connectedAddress);
    toast.success("Using connected wallet");
  }

  async function onCopyJson() {
    if (!rawResponse) return;
    try {
      await navigator.clipboard.writeText(JSON.stringify(rawResponse, null, 2));
      toast.success("Copied");
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function onExecute() {
    setLoading(true);
    setError(null);
    setResult(null);
    setRawResponse(null);
    setExecutionLog([]);

    const startedAt = Date.now();
    setExecutionLog((prev) => [...prev, `Sending ${tool}...`]);

    try {
      const args: Record<string, unknown> = (() => {
        switch (tool) {
          case "get_account_summary":
          case "get_defi_positions":
            return { address, simple_mode: simpleMode };
          case "decode_transaction":
            return { tx_hash: txHash, simple_mode: simpleMode };
          case "simulate_transaction":
            return {
              from: address,
              to: simulateTo,
              data: simulateData,
              value: simulateValue,
              simple_mode: simpleMode,
            };
          case "search_contract":
            return { query: searchQuery, limit: searchLimit };
          case "construct_swap_tx":
            return {
              from: address,
              token_in: swapTokenIn,
              token_out: swapTokenOut,
              amount_in: swapAmountIn,
              slippage_bps: swapSlippageBps,
            };
        }
      })();

      addLog({ level: "info", tool, message: "Request started" });

      const { traceId, durationMs, response } = await mcpCallRaw<ToolResult>(
        tool,
        args,
        { apiKey },
      );

      setRawResponse(response);
      setResult((response.result ?? null) as ToolResult | null);

      const meta = extractMeta(response.result);
      const serverLatency = meta?.latency_ms;
      const totalMs = Date.now() - startedAt;
      setExecutionLog((prev) => [
        ...prev,
        `Done (server: ${serverLatency ?? "n/a"}ms, client: ${Math.max(totalMs, durationMs)}ms)`,
        `trace_id: ${traceId}`,
      ]);

      addLog({
        level: "info",
        tool,
        message: `Done (${serverLatency ?? Math.round(durationMs)}ms)`,
        traceId,
      });
      addLatency({
        ts: Date.now(),
        tool,
        latencyMs:
          typeof serverLatency === "number"
            ? serverLatency
            : Math.round(durationMs),
        status: "success",
      });
    } catch (err) {
      const { message, code, traceId } = getMcpErrorMessage(err);
      const detailParts: string[] = [];
      if (typeof code === "number") detailParts.push(`code: ${code}`);
      if (typeof traceId === "string") detailParts.push(`trace_id: ${traceId}`);
      const detail = detailParts.length ? ` (${detailParts.join(", ")})` : "";
      const combined = `${message}${detail}`;

      toast.error(message);
      setError(combined);
      setExecutionLog((prev) => [...prev, `Error: ${combined}`]);

      addLog({ level: "error", tool, message: combined, traceId });
      addLatency({
        ts: Date.now(),
        tool,
        latencyMs: Math.round(Date.now() - startedAt),
        status: "error",
      });
    } finally {
      setLoading(false);
    }
  }

  const meta = extractMeta(result);

  return (
    <div className="space-y-8">
      <P5Title subTitle="Test MCP tools against your CroLens API.">
        PLAYGROUND
      </P5Title>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-12">
        {/* 工具列表 */}
        <div className="lg:col-span-3">
          <P5Card title="TOOLS" className="h-[400px] lg:h-full">
            <ToolSelector
              value={tool}
              options={TOOL_OPTIONS.map((t) => ({
                value: t.value,
                label: t.label,
              }))}
              onValueChange={(v) => setTool(v as ToolName)}
            />
          </P5Card>
        </div>

        {/* 主输入区 */}
        <div className="lg:col-span-5 flex flex-col gap-6">
          <P5Card title="INPUT PARAMETERS" className="flex-1">
            <div className="space-y-4" aria-busy={loading}>
              {/* 选中工具提示 */}
              <div className="bg-[#00C853]/10 border border-[#00C853]/30 p-3 text-xs font-mono text-[#00C853] flex items-start gap-2">
                <LayoutDashboard size={14} className="mt-0.5" />
                <span>SELECTED: {TOOL_OPTIONS.find(t => t.value === tool)?.label}</span>
              </div>

              {/* 移动端工具选择器 */}
              <div className="md:hidden">
                <Select
                  value={tool}
                  onValueChange={(v) => setTool(v as ToolName)}
                >
                  <SelectTrigger className="bg-[#242424] border-[#333] text-white">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {TOOL_OPTIONS.map((t) => (
                      <SelectItem key={t.value} value={t.value}>
                        {t.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="text-xs text-[#A3A3A3]">{toolHelp}</div>

              {needsAddress ? (
                <div className="space-y-2">
                  <P5Input
                    id={addressInputId}
                    label={tool === "construct_swap_tx" ? "FROM ADDRESS" : "TARGET ADDRESS"}
                    value={address}
                    onChange={(e) => setAddress(e.target.value)}
                    placeholder="0x..."
                    aria-invalid={
                      address.trim().length > 0 && !isAddress(address)
                    }
                    rightElement={
                      <div className="flex items-center gap-1">
                        <button
                          type="button"
                          onClick={onPaste}
                          className="text-[#A3A3A3] hover:text-white transition-colors"
                          title="Paste"
                        >
                          <ClipboardPaste size={16} />
                        </button>
                        <button
                          type="button"
                          onClick={() => setAddress("")}
                          className="text-[#A3A3A3] hover:text-white transition-colors"
                          title="Clear"
                        >
                          <Eraser size={16} />
                        </button>
                      </div>
                    }
                    className="mb-0"
                  />
                  <div className="flex flex-wrap items-center gap-2 text-xs text-[#A3A3A3] ml-1">
                    <span className="inline-flex items-center gap-1">
                      <Link2 className="h-3.5 w-3.5" />
                      Wallet:
                    </span>
                    {isConnected && connectedAddress ? (
                      <>
                        <span className="font-mono">{connectedAddress.slice(0, 10)}...</span>
                        <button
                          type="button"
                          onClick={useConnected}
                          className="text-[#D90018] hover:text-white transition-colors uppercase text-xs"
                        >
                          Use
                        </button>
                      </>
                    ) : (
                      <span>not connected</span>
                    )}
                  </div>
                  {address.trim().length > 0 && !isAddress(address) ? (
                    <div className="text-xs text-[#FF4444] ml-1" role="alert">
                      Invalid address
                    </div>
                  ) : null}
                </div>
              ) : null}

              {tool === "decode_transaction" ? (
                <div className="space-y-2">
                  <P5Input
                    id={txHashInputId}
                    label="TX HASH"
                    value={txHash}
                    onChange={(e) => setTxHash(e.target.value)}
                    placeholder="0x..."
                    aria-invalid={txHash.trim().length > 0 && !isTxHash(txHash)}
                    className="mb-0"
                  />
                  {txHash.trim().length > 0 && !isTxHash(txHash) ? (
                    <div className="text-xs text-[#FF4444] ml-1" role="alert">
                      Invalid tx hash
                    </div>
                  ) : null}
                </div>
              ) : null}

              {tool === "simulate_transaction" ? (
                <div className="space-y-4">
                  <P5Input
                    id={simulateToInputId}
                    label="TO"
                    value={simulateTo}
                    onChange={(e) => setSimulateTo(e.target.value)}
                    placeholder="0x..."
                    aria-invalid={
                      simulateTo.trim().length > 0 && !isAddress(simulateTo)
                    }
                    className="mb-0"
                  />
                  <div className="mb-4">
                    <label className="block font-bebas tracking-wider text-[#A3A3A3] mb-1 ml-1 text-lg">
                      DATA
                    </label>
                    <textarea
                      id={simulateDataInputId}
                      value={simulateData}
                      onChange={(e) => setSimulateData(e.target.value)}
                      rows={3}
                      placeholder="0x..."
                      className="w-full bg-[#242424] border-2 border-[#333] focus:border-[#D90018] focus:shadow-[0_0_10px_rgba(217,0,24,0.3)] transition-all rounded-sm text-white font-mono px-4 py-3 outline-none placeholder-[#555]"
                      spellCheck="false"
                      aria-invalid={
                        simulateData.trim().length > 0 && !isHexData(simulateData)
                      }
                    />
                    {simulateData.trim().length > 0 &&
                    !isHexData(simulateData) ? (
                      <div className="text-xs text-[#FF4444] ml-1 mt-1" role="alert">
                        Invalid hex data
                      </div>
                    ) : null}
                  </div>
                  <P5Input
                    id={simulateValueInputId}
                    label="VALUE"
                    value={simulateValue}
                    onChange={(e) => setSimulateValue(e.target.value)}
                    placeholder="0"
                    className="mb-0"
                  />
                  <div className="text-xs text-[#A3A3A3] ml-1">
                    Decimal or 0x-prefixed hex.
                  </div>
                </div>
              ) : null}

              {tool === "search_contract" ? (
                <div className="space-y-4">
                  <P5Input
                    id={searchQueryInputId}
                    label="QUERY"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    placeholder="VVS Router"
                    className="mb-0"
                  />
                  <div className="mb-4">
                    <label htmlFor={searchLimitInputId} className="block font-bebas tracking-wider text-[#A3A3A3] mb-1 ml-1 text-lg">
                      LIMIT: {searchLimit}
                    </label>
                    <input
                      id={searchLimitInputId}
                      type="range"
                      min={1}
                      max={50}
                      value={searchLimit}
                      onChange={(e) => setSearchLimit(Number(e.target.value))}
                      className="crolens-range w-full"
                    />
                  </div>
                </div>
              ) : null}

              {tool === "construct_swap_tx" ? (
                <div className="space-y-4">
                  <div className="grid grid-cols-2 gap-3">
                    <P5Input
                      id={swapTokenInInputId}
                      label="TOKEN IN"
                      value={swapTokenIn}
                      onChange={(e) => setSwapTokenIn(e.target.value)}
                      placeholder="CRO"
                      className="mb-0"
                    />
                    <P5Input
                      id={swapTokenOutInputId}
                      label="TOKEN OUT"
                      value={swapTokenOut}
                      onChange={(e) => setSwapTokenOut(e.target.value)}
                      placeholder="USDC"
                      className="mb-0"
                    />
                  </div>
                  <P5Input
                    id={swapAmountInInputId}
                    label="AMOUNT IN"
                    value={swapAmountIn}
                    onChange={(e) => setSwapAmountIn(e.target.value)}
                    placeholder="1000000000000000000"
                    className="mb-0"
                  />
                  <div className="text-xs text-[#A3A3A3] ml-1">
                    Integer (wei). Backend currently expects a decimal integer.
                  </div>
                  <div className="mb-4">
                    <label htmlFor={swapSlippageInputId} className="block font-bebas tracking-wider text-[#A3A3A3] mb-1 ml-1 text-lg">
                      SLIPPAGE (BPS): {swapSlippageBps}
                    </label>
                    <input
                      id={swapSlippageInputId}
                      type="range"
                      min={0}
                      max={5000}
                      value={swapSlippageBps}
                      onChange={(e) => setSwapSlippageBps(Number(e.target.value))}
                      className="crolens-range w-full"
                    />
                  </div>
                </div>
              ) : null}

              {supportsSimpleMode ? (
                <div className="flex items-center justify-between py-2 border-t border-[#333]">
                  <label htmlFor={simpleModeSwitchId} className="font-bebas tracking-wider text-[#A3A3A3] text-lg">
                    SIMPLE_MODE
                  </label>
                  <Switch
                    id={simpleModeSwitchId}
                    checked={simpleMode}
                    onCheckedChange={setSimpleMode}
                  />
                </div>
              ) : null}

              <div className="border-t border-[#333] pt-4 space-y-3">
                <div className="text-xs text-[#555] font-mono">
                  x-api-key: {apiKey.length > 0 ? "set" : "missing"}
                </div>
                <P5Button
                  onClick={onExecute}
                  disabled={!canExecute}
                  loading={loading}
                  icon={Play}
                  className="w-full shadow-lg"
                >
                  {loading ? 'PROCESSING...' : 'EXECUTE FUNCTION'}
                </P5Button>
              </div>
            </div>
          </P5Card>
        </div>

        {/* 输出结果 */}
        <div className="lg:col-span-4">
          <P5Card
            title="OUTPUT JSON"
            className="h-full min-h-[400px]"
            headerAction={
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={() => setShowRaw(!showRaw)}
                  className={`text-xs font-mono uppercase transition-colors ${showRaw ? 'text-white' : 'text-[#A3A3A3] hover:text-white'}`}
                >
                  [{showRaw ? 'FORMATTED' : 'RAW'}]
                </button>
                <button
                  type="button"
                  onClick={onCopyJson}
                  disabled={!rawResponse}
                  className="text-xs font-mono text-[#A3A3A3] hover:text-white uppercase disabled:opacity-50"
                >
                  [COPY]
                </button>
              </div>
            }
          >
            <div className="h-full overflow-auto" aria-busy={loading}>
              {error ? (
                <div
                  className="rounded-sm border border-[#FF4444]/40 bg-[#FF4444]/10 p-4 text-sm"
                  role="alert"
                >
                  <div className="font-bebas text-[#FF4444] tracking-wider">ERROR</div>
                  <div className="mt-1 text-[#A3A3A3] font-mono text-xs">{error}</div>
                </div>
              ) : null}

              {renderMeta(meta)}

              {loading ? (
                <div className="flex items-center justify-center h-full gap-2 text-[#D90018] animate-pulse">
                  <Activity className="animate-spin" /> ESTABLISHING LINK...
                </div>
              ) : showRaw ? (
                <CodeBlock
                  code={
                    rawResponse
                      ? JSON.stringify(rawResponse, null, 2)
                      : "No response yet."
                  }
                  aria-label="Raw JSON response"
                />
              ) : result ? (
                <FormattedResult tool={tool} result={result} />
              ) : (
                <div className="h-full flex flex-col items-center justify-center text-[#555] opacity-50 min-h-[200px]">
                  <Box size={48} strokeWidth={1} className="mb-2" />
                  <span className="font-bebas tracking-wide text-xl">AWAITING INPUT</span>
                </div>
              )}
            </div>
          </P5Card>
        </div>
      </div>
    </div>
  );
}

function FormattedResult({
  tool,
  result,
}: {
  tool: ToolName;
  result: ToolResult | null;
}) {
  if (!result) {
    return <div className="text-sm text-[#A3A3A3]">No results yet.</div>;
  }

  if ("text" in result) {
    return (
      <div className="space-y-2">
        <div className="font-bebas tracking-wider text-white">SUMMARY</div>
        <div className="bg-black/50 border border-[#333] p-4 text-sm leading-relaxed whitespace-pre-wrap text-[#A3A3A3] font-mono">
          {result.text}
        </div>
      </div>
    );
  }

  if (tool === "get_account_summary" && "wallet" in result) {
    return <AccountSummaryView value={result} />;
  }
  if (tool === "get_defi_positions" && "vvs" in result) {
    return <DefiPositionsView value={result} />;
  }
  if (tool === "decode_transaction" && "hash" in result) {
    return <DecodedTxView value={result} />;
  }
  if (tool === "simulate_transaction" && "success" in result) {
    return <SimulationView value={result} />;
  }
  if (tool === "search_contract" && "results" in result) {
    return <ContractSearchView value={result} />;
  }
  if (tool === "construct_swap_tx" && "steps" in result) {
    return <SwapPipelineView value={result} />;
  }

  return (
    <div className="bg-black/50 border border-[#333] p-4 text-sm text-[#A3A3A3]">
      Unsupported result shape.
    </div>
  );
}

function parseDecimal(value: string | null | undefined) {
  if (!value) return 0;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function formatUsdAmount(value: number) {
  if (!Number.isFinite(value)) return "—";
  return `$${value.toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })}`;
}

function formatUsd(value: string | null | undefined) {
  if (!value) return "—";
  return formatUsdAmount(parseDecimal(value));
}

function AccountSummaryView({ value }: { value: AccountSummary }) {
  const walletRows = React.useMemo(
    () =>
      value.wallet
        .map((token) => ({
          token,
          valueUsd: parseDecimal(token.value_usd),
        }))
        .sort((a, b) => b.valueUsd - a.valueUsd),
    [value.wallet],
  );
  const totalWalletValueUsd = walletRows.reduce(
    (acc, row) => acc + row.valueUsd,
    0,
  );

  return (
    <div className="space-y-4">
      <div className="bg-black/50 border border-[#333] p-4">
        <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
          <div className="space-y-1">
            <div className="font-bebas tracking-wider text-white">WALLET SUMMARY</div>
            <div className="text-xs text-[#A3A3A3]">Address</div>
            <div className="break-all font-mono text-xs text-white">
              {value.address}
            </div>
          </div>

          <div className="bg-[#1A1A1A] border-l-2 border-[#D90018] p-3 text-right">
            <div className="text-xs text-[#A3A3A3]">Net Worth</div>
            <div className="mt-1 text-2xl font-bebas tabular-nums text-white">
              {formatUsd(value.total_net_worth_usd)}
            </div>
            <div className="mt-1 text-xs text-[#A3A3A3]">
              DeFi: {formatUsd(value.defi_summary.total_defi_value_usd)}
            </div>
          </div>
        </div>
      </div>

      <div className="overflow-x-auto bg-black/50 border border-[#333]">
        <table className="w-full text-sm">
          <thead className="text-left text-xs text-[#A3A3A3] border-b border-[#333]">
            <tr>
              <th className="px-4 py-2">ASSET</th>
              <th className="px-4 py-2 text-right">BALANCE</th>
              <th className="px-4 py-2 text-right">VALUE</th>
              <th className="px-4 py-2 text-right">SHARE</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#333]">
            {walletRows.map(({ token, valueUsd }) => {
              const share =
                totalWalletValueUsd > 0 ? valueUsd / totalWalletValueUsd : 0;
              const percent = Math.round(share * 100);

              return (
                <tr key={token.token_address} className="hover:bg-white/5">
                  <td className="px-4 py-2 font-medium text-white">{token.symbol}</td>
                  <td className="px-4 py-2 text-right font-mono text-[#A3A3A3]">
                    {token.balance_formatted}
                  </td>
                  <td className="px-4 py-2 text-right tabular-nums text-white">
                    {formatUsd(token.value_usd)}
                  </td>
                  <td className="px-4 py-2">
                    <div className="flex items-center justify-end gap-2">
                      <div className="h-1 w-[60px] overflow-hidden bg-[#333]">
                        <div
                          className="h-full bg-[#D90018] transition-[width] duration-300"
                          style={{ width: `${percent}%` }}
                        />
                      </div>
                      <span className="w-10 text-right text-xs tabular-nums text-[#A3A3A3]">
                        {percent}%
                      </span>
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="bg-[#1A1A1A] p-4 border-l-2 border-[#D90018]">
          <div className="text-xs text-[#A3A3A3] font-bebas tracking-wider">VVS LIQUIDITY</div>
          <div className="mt-1 text-lg font-bebas tabular-nums text-white">
            {formatUsd(value.defi_summary.vvs_liquidity_usd)}
          </div>
        </div>
        <div className="bg-[#1A1A1A] p-4 border-l-2 border-[#D90018]">
          <div className="text-xs text-[#A3A3A3] font-bebas tracking-wider">TECTONIC NET</div>
          <div className="mt-1 text-lg font-bebas tabular-nums text-white">
            {formatUsdAmount(
              parseDecimal(value.defi_summary.tectonic_supply_usd) -
                parseDecimal(value.defi_summary.tectonic_borrow_usd),
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function DefiPositionsView({ value }: { value: DefiPositions }) {
  return (
    <div className="space-y-4">
      <div className="bg-black/50 border border-[#333] p-4">
        <div className="font-bebas tracking-wider text-white mb-2">VVS FINANCE</div>
        <div className="text-xs text-[#A3A3A3] mb-3">
          Liquidity ${value.vvs.total_liquidity_usd} • Pending Rewards $
          {value.vvs.total_pending_rewards_usd}
        </div>
        {value.vvs.positions.length === 0 ? (
          <div className="text-sm text-[#A3A3A3]">No VVS positions found.</div>
        ) : (
          <div className="space-y-2">
            {value.vvs.positions.map((p) => (
              <div
                key={p.pool_id}
                className="bg-[#1A1A1A] border-l-2 border-[#D90018] p-3"
              >
                <div className="flex items-center justify-between gap-2">
                  <div className="font-medium text-white">{p.pool_name}</div>
                  <div className="flex items-center gap-2">
                    {p.liquidity_usd ? (
                      <Badge variant="default">${p.liquidity_usd}</Badge>
                    ) : (
                      <Badge variant="secondary">USD n/a</Badge>
                    )}
                    <Badge variant="success">
                      {p.pending_vvs_formatted} VVS
                    </Badge>
                  </div>
                </div>
                <div className="mt-2 grid grid-cols-2 gap-3 text-xs text-[#A3A3A3]">
                  <div>
                    <div className="font-mono">{p.token0.amount_formatted}</div>
                    <div>{p.token0.symbol}</div>
                  </div>
                  <div>
                    <div className="font-mono">{p.token1.amount_formatted}</div>
                    <div>{p.token1.symbol}</div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="bg-black/50 border border-[#333] p-4">
        <div className="font-bebas tracking-wider text-white mb-2">TECTONIC</div>
        <div className="text-xs text-[#A3A3A3] mb-3">
          Supply ${value.tectonic.total_supply_usd} • Borrow $
          {value.tectonic.total_borrow_usd} • Health{" "}
          {value.tectonic.health_factor}
        </div>
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div>
            <div className="mb-2 text-sm font-bebas tracking-wider text-[#A3A3A3]">SUPPLIES</div>
            {value.tectonic.supplies.length === 0 ? (
              <div className="text-sm text-[#555]">None</div>
            ) : (
              <div className="space-y-2">
                {value.tectonic.supplies.map((s) => (
                  <div
                    key={s.market_address}
                    className="bg-[#1A1A1A] border-l-2 border-[#00FF41] p-3"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <div className="font-medium text-white">{s.asset_symbol}</div>
                      <div className="text-sm text-[#00FF41]">
                        {s.supply_balance_usd
                          ? `$${s.supply_balance_usd}`
                          : "—"}
                      </div>
                    </div>
                    <div className="mt-1 font-mono text-xs text-[#A3A3A3]">
                      {s.supply_balance}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
          <div>
            <div className="mb-2 text-sm font-bebas tracking-wider text-[#A3A3A3]">BORROWS</div>
            {value.tectonic.borrows.length === 0 ? (
              <div className="text-sm text-[#555]">None</div>
            ) : (
              <div className="space-y-2">
                {value.tectonic.borrows.map((b) => (
                  <div
                    key={b.market_address}
                    className="bg-[#1A1A1A] border-l-2 border-[#FFD700] p-3"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <div className="font-medium text-white">{b.asset_symbol}</div>
                      <div className="text-sm text-[#FFD700]">
                        {b.borrow_balance_usd
                          ? `$${b.borrow_balance_usd}`
                          : "—"}
                      </div>
                    </div>
                    <div className="mt-1 font-mono text-xs text-[#A3A3A3]">
                      {b.borrow_balance}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function DecodedTxView({ value }: { value: DecodedTransaction }) {
  return (
    <div className="bg-black/50 border border-[#333] p-4 space-y-3">
      <div className="font-bebas tracking-wider text-white">DECODED TRANSACTION</div>
      <div className="font-mono text-xs text-[#A3A3A3] break-all">{value.hash}</div>
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant="default">{value.action}</Badge>
        <Badge variant="secondary">{value.decoded.method_name}</Badge>
        <Badge variant="warning">{value.status}</Badge>
      </div>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        <div className="bg-[#1A1A1A] border-l-2 border-[#D90018] p-3">
          <div className="text-xs text-[#A3A3A3] font-bebas tracking-wider">FROM</div>
          <div className="font-mono text-xs text-white mt-1">{value.from}</div>
        </div>
        <div className="bg-[#1A1A1A] border-l-2 border-[#D90018] p-3">
          <div className="text-xs text-[#A3A3A3] font-bebas tracking-wider">TO</div>
          <div className="font-mono text-xs text-white mt-1">{value.to}</div>
        </div>
      </div>
      <div className="bg-[#1A1A1A] border-l-2 border-[#D90018] p-3">
        <div className="text-xs text-[#A3A3A3] font-bebas tracking-wider">GAS USED</div>
        <div className="font-mono text-sm text-white mt-1">{value.gas_used}</div>
      </div>
    </div>
  );
}

function SimulationView({ value }: { value: SimulationResult }) {
  const ok = !!value.success;
  return (
    <div className="bg-black/50 border border-[#333] p-4 space-y-4">
      <div className="font-bebas tracking-wider text-white">SIMULATION</div>
      <div className="text-xs text-[#A3A3A3]">Tenderly-backed when configured</div>
      <div className="flex items-center gap-2">
        <Badge variant={ok ? "success" : "destructive"}>
          {ok ? "Success" : "Failed"}
        </Badge>
        {value.simulation_available === false ? (
          <Badge variant="warning">Unavailable</Badge>
        ) : null}
        {value.gas_estimated ? (
          <Badge variant="secondary">Gas: {value.gas_estimated}</Badge>
        ) : null}
      </div>

      {Array.isArray(value.state_changes) &&
      value.state_changes.length > 0 ? (
        <div className="space-y-2">
          <div className="font-bebas tracking-wider text-[#A3A3A3]">STATE CHANGES</div>
          <div className="space-y-2">
            {value.state_changes.map((c, idx) => (
              <div
                key={idx}
                className="bg-[#1A1A1A] border-l-2 border-[#D90018] p-3"
              >
                <div className="text-sm font-medium text-white">{c.description}</div>
                <div className="mt-1 grid grid-cols-1 gap-2 text-xs text-[#A3A3A3] md:grid-cols-2 font-mono">
                  <div>from: {c.from}</div>
                  <div>to: {c.to}</div>
                  <div>amount: {c.amount}</div>
                  <div>token: {c.token}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div className="text-sm text-[#555]">No decoded state changes.</div>
      )}
    </div>
  );
}

function ContractSearchView({ value }: { value: ContractSearchResponse }) {
  return (
    <div className="bg-black/50 border border-[#333] p-4 space-y-3">
      <div className="font-bebas tracking-wider text-white">RESULTS</div>
      <div className="text-xs text-[#A3A3A3]">{value.results.length} match(es)</div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="text-left text-xs text-[#A3A3A3] border-b border-[#333]">
            <tr>
              <th className="py-2">NAME</th>
              <th className="py-2">ADDRESS</th>
              <th className="py-2">TYPE</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#333]">
            {value.results.map((r, idx) => (
              <tr key={r.address ?? `${r.name ?? "row"}-${idx}`} className="hover:bg-white/5">
                <td className="py-2 font-medium text-white">{r.name ?? "—"}</td>
                <td className="py-2 font-mono text-xs text-[#A3A3A3]">{r.address ?? "—"}</td>
                <td className="py-2 text-[#A3A3A3]">{r.type ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function SwapPipelineView({ value }: { value: SwapPipeline }) {
  return (
    <div className="space-y-4">
      <div className="bg-black/50 border border-[#333] p-4">
        <div className="font-bebas tracking-wider text-white">SWAP PIPELINE</div>
        <div className="font-mono text-xs text-[#A3A3A3] mt-1">{value.operation_id}</div>
        <div className="flex flex-wrap items-center gap-2 mt-3">
          <Badge variant="default">estimated_out: {value.estimated_out}</Badge>
          <Badge variant="secondary">minimum_out: {value.minimum_out}</Badge>
          <Badge variant={value.simulation_verified ? "success" : "warning"}>
            simulation_verified: {String(value.simulation_verified)}
          </Badge>
        </div>
      </div>

      <div className="bg-black/50 border border-[#333] p-4">
        <div className="font-bebas tracking-wider text-white mb-2">STEPS</div>
        <div className="text-xs text-[#A3A3A3] mb-3">Execute in order</div>
        <div className="space-y-3">
          {value.steps.map((s) => (
            <div
              key={s.step_index}
              className="bg-[#1A1A1A] border-l-2 border-[#D90018] p-3"
            >
              <div className="flex items-center justify-between gap-2">
                <div className="font-medium text-white">
                  {s.step_index}. {s.type}
                </div>
                <Badge variant={s.status === "pending" ? "default" : "warning"}>
                  {s.status}
                </Badge>
              </div>
              <div className="mt-1 text-sm text-[#A3A3A3]">
                {s.description}
              </div>
              <div className="mt-2 space-y-1 text-xs text-[#555] font-mono">
                <div>to: {s.tx_data.to}</div>
                <div>value: {s.tx_data.value}</div>
                <div className="break-all">data: {s.tx_data.data}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

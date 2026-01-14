import * as React from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { Copy, Eye, EyeOff, RefreshCcw, Wallet, X } from "lucide-react";
import { formatUnits } from "viem";
import {
  useAccount,
  useSendTransaction,
  useSwitchChain,
  useWaitForTransactionReceipt,
} from "wagmi";
import { cronos } from "wagmi/chains";
import { P5Title, P5Card, P5Button } from "@/components/p5";
import { CodeBlock } from "@/components/ui/code-block";
import {
  fetchX402Quote,
  fetchX402Status,
  getDefaultApiUrl,
  HttpApiError,
  verifyX402Payment,
} from "@/lib/api";
import { createDemoApiKey, useAppStore } from "@/stores/app";

function maskApiKey(apiKey: string) {
  const trimmed = apiKey.trim();
  if (trimmed.length <= 12) return trimmed;
  return `${trimmed.slice(0, 8)}…${trimmed.slice(-4)}`;
}

function buildClaudeDesktopConfig(apiKey: string, apiUrl: string) {
  return {
    mcpServers: {
      crolens: {
        command: "npx",
        args: ["-y", "crolens-mcp"],
        env: {
          CROLENS_API_KEY: apiKey,
          CROLENS_API_URL: apiUrl,
        },
      },
    },
  };
}

function formatCroFromWei(valueWei: string) {
  try {
    const formatted = formatUnits(BigInt(valueWei), 18);
    const asNumber = Number(formatted);
    if (Number.isFinite(asNumber)) {
      return asNumber.toFixed(4).replace(/\.?0+$/, "");
    }
    return formatted;
  } catch {
    return valueWei;
  }
}

function stringifyError(err: unknown) {
  if (err instanceof HttpApiError) return err.message;
  if (err instanceof Error) return err.message;
  return String(err);
}

// P5 风格 Badge
function P5Badge({
  children,
  variant = "default"
}: {
  children: React.ReactNode;
  variant?: "default" | "success" | "warning" | "destructive" | "pro";
}) {
  const colorMap = {
    default: "bg-[#333] text-[#A3A3A3]",
    success: "bg-[#00FF41]/20 text-[#00FF41] border border-[#00FF41]/30",
    warning: "bg-[#FFD700]/20 text-[#FFD700] border border-[#FFD700]/30",
    destructive: "bg-[#FF4444]/20 text-[#FF4444] border border-[#FF4444]/30",
    pro: "bg-[#D90018] text-white",
  };

  return (
    <span className={`inline-flex items-center px-2 py-0.5 text-xs font-mono tracking-wider ${colorMap[variant]}`}>
      {children}
    </span>
  );
}

// P5 风格 Dialog
function P5Dialog({
  open,
  onClose,
  title,
  description,
  children,
  footer,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
}) {
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/80 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Dialog */}
      <div
        className="relative w-full max-w-lg mx-4 bg-[#121212] border-2 border-[#333] shadow-[4px_4px_0px_0px_#D90018]"
        style={{ clipPath: 'polygon(0 0, calc(100% - 20px) 0, 100% 20px, 100% 100%, 0 100%)' }}
      >
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-[#333]">
          <div>
            <div className="relative inline-block transform -skew-x-6 bg-[#D90018] px-3 py-1">
              <h2 className="transform skew-x-6 font-bebas text-lg tracking-wider text-white">
                {title}
              </h2>
            </div>
            {description && (
              <p className="mt-2 text-sm text-[#A3A3A3] font-inter">{description}</p>
            )}
          </div>
          <button
            onClick={onClose}
            className="text-[#A3A3A3] hover:text-white transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-4 max-h-[60vh] overflow-y-auto">
          {children}
        </div>

        {/* Footer */}
        {footer && (
          <div className="flex justify-end gap-3 p-4 border-t border-[#333]">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}

// P5 风格 Progress Bar
function P5Progress({ value, max }: { value: number; max: number }) {
  const percent = Math.min((value / max) * 100, 100);

  return (
    <div className="relative h-3 bg-[#333] overflow-hidden">
      <div
        className="h-full bg-[#D90018] transition-all duration-500"
        style={{ width: `${percent}%` }}
      />
      {/* 扫描线效果 */}
      <div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent animate-pulse" />
    </div>
  );
}

// P5 风格 Step Card
function StepCard({
  step,
  title,
  children,
}: {
  step: number;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="bg-[#1A1A1A] border-l-2 border-[#D90018] p-3">
      <div className="flex items-center gap-2 mb-2">
        <span className="w-6 h-6 bg-[#D90018] text-white font-bebas text-sm flex items-center justify-center">
          {step}
        </span>
        <span className="font-bebas text-white tracking-wider">{title}</span>
      </div>
      <div className="text-sm text-[#A3A3A3] font-mono">
        {children}
      </div>
    </div>
  );
}

export function ConsolePage() {
  const apiKey = useAppStore((s) => s.apiKey);
  const setApiKey = useAppStore((s) => s.setApiKey);
  const apiKeyVisible = useAppStore((s) => s.apiKeyVisible);
  const setApiKeyVisible = useAppStore((s) => s.setApiKeyVisible);

  const tier = useAppStore((s) => s.tier);
  const credits = useAppStore((s) => s.credits);
  const planCredits = useAppStore((s) => s.planCredits);
  const setBilling = useAppStore((s) => s.setBilling);

  const apiUrl = getDefaultApiUrl();
  const isE2E =
    typeof import.meta.env.VITE_E2E === "string" &&
    import.meta.env.VITE_E2E === "true";
  React.useId();

  const [regenOpen, setRegenOpen] = React.useState(false);
  const [topUpOpen, setTopUpOpen] = React.useState(false);
  const [showRawConfig, setShowRawConfig] = React.useState(false);
  const [paymentTxHash, setPaymentTxHash] = React.useState<
    `0x${string}` | null
  >(null);
  const [verifyStatus, setVerifyStatus] = React.useState<string | null>(null);
  const [verifyError, setVerifyError] = React.useState<string | null>(null);

  const { address, isConnected, chainId } = useAccount();
  const { switchChainAsync, isPending: isSwitching } = useSwitchChain();
  const { sendTransactionAsync, isPending: isSending } = useSendTransaction();
  const receipt = useWaitForTransactionReceipt({
    hash: paymentTxHash ?? undefined,
    query: { enabled: !!paymentTxHash && !isE2E },
  });

  const statusQuery = useQuery({
    queryKey: ["x402_status", apiUrl, apiKey],
    queryFn: () => fetchX402Status({ apiUrl, apiKey }),
    enabled: apiKey.trim().length > 0,
    retry: false,
    refetchInterval: 30_000,
  });

  React.useEffect(() => {
    if (!statusQuery.data) return;
    setBilling({
      tier: statusQuery.data.tier,
      credits: statusQuery.data.credits,
    });
  }, [setBilling, statusQuery.data]);

  const quoteQuery = useQuery({
    queryKey: ["x402_quote", apiUrl],
    queryFn: () => fetchX402Quote(apiUrl),
    enabled: topUpOpen,
    retry: false,
    staleTime: 60_000,
  });

  React.useEffect(() => {
    if (topUpOpen) return;
    setPaymentTxHash(null);
    setVerifyStatus(null);
    setVerifyError(null);
  }, [topUpOpen]);

  const remaining = Math.max(credits, 0);

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast.success("Copied");
    } catch (err) {
      toast.error(String(err));
    }
  }

  function regenerate() {
    const next = createDemoApiKey();
    setApiKey(next);
    setRegenOpen(false);
    toast.success("API key regenerated");
  }

  const isCronos = chainId === cronos.id;

  async function switchToCronos() {
    try {
      await switchChainAsync({ chainId: cronos.id });
    } catch (err) {
      toast.error(stringifyError(err));
    }
  }

  async function sendTopUpTransaction() {
    const quote = quoteQuery.data;
    if (!quote) {
      toast.error("Failed to load top-up quote. Please retry.");
      return;
    }

    if (!isConnected || !address) {
      toast.error("Please connect your wallet first.");
      return;
    }

    if (!isCronos) {
      toast.error("Please switch to Cronos mainnet.");
      return;
    }

    setVerifyError(null);
    setVerifyStatus(null);

    try {
      const hash = await sendTransactionAsync({
        to: quote.payment_address as `0x${string}`,
        value: BigInt(quote.amount_wei),
      });
      setPaymentTxHash(hash);
      toast.success("Transaction sent. Waiting for confirmation...");
    } catch (err) {
      toast.error(stringifyError(err));
    }
  }

  React.useEffect(() => {
    if (!paymentTxHash) return;

    let cancelled = false;
    let attempts = 0;
    let interval: number | null = null;

    const run = async () => {
      if (cancelled) return;
      attempts += 1;

      try {
        const resp = await verifyX402Payment(paymentTxHash, { apiUrl, apiKey });
        if (cancelled) return;

        setVerifyStatus(resp.status);
        setVerifyError(null);

        if (resp.status === "credited" || resp.status === "already_credited") {
          if (
            typeof resp.credits === "number" &&
            typeof resp.tier === "string"
          ) {
            setBilling({ tier: resp.tier, credits: resp.credits });
          }

          toast.success(
            resp.status === "credited"
              ? "Top-up credited. Credits updated."
              : "Transaction already processed. Credits synced.",
          );

          if (interval) window.clearInterval(interval);
          interval = null;
          return;
        }

        if (attempts >= 40) {
          toast.warning(
            "Verification timed out. Please refresh credits later.",
          );
          if (interval) window.clearInterval(interval);
          interval = null;
        }
      } catch (err) {
        if (cancelled) return;
        const msg = stringifyError(err);
        setVerifyError(msg);
        toast.error(msg);
        if (interval) window.clearInterval(interval);
        interval = null;
      }
    };

    void run();
    interval = window.setInterval(run, 5000);

    return () => {
      cancelled = true;
      if (interval) window.clearInterval(interval);
    };
  }, [apiKey, apiUrl, paymentTxHash, setBilling]);

  const configObject = React.useMemo(
    () => buildClaudeDesktopConfig(apiKey, apiUrl),
    [apiKey, apiUrl],
  );
  const configJsonFormatted = React.useMemo(
    () => JSON.stringify(configObject, null, 2),
    [configObject],
  );
  const configJsonRaw = React.useMemo(
    () => JSON.stringify(configObject),
    [configObject],
  );
  const configJson = showRawConfig ? configJsonRaw : configJsonFormatted;

  const quote = quoteQuery.data;
  const quoteAmountCro =
    typeof quote?.amount_wei === "string"
      ? `${formatCroFromWei(quote.amount_wei)} CRO`
      : null;
  const txLabel = !paymentTxHash
    ? "NOT_SENT"
    : receipt.isSuccess
      ? "CONFIRMED"
      : receipt.isError
        ? "FAILED"
        : "PENDING";

  return (
    <div className="mx-auto max-w-[900px] space-y-8">
      {/* Header */}
      <div className="flex flex-wrap items-start justify-between gap-4">
        <P5Title subTitle="API key, usage, and Claude Desktop config.">
          CONSOLE
        </P5Title>
        <P5Badge variant={tier === "pro" ? "pro" : "default"}>
          ◆ PLAN: {tier.toUpperCase()}
        </P5Badge>
      </div>

      <div className="space-y-6">
        {/* API Key Card */}
        <P5Card title="API KEY">
          <div className="space-y-4">
            <div className="text-xs text-[#555] font-mono mb-2">
              Stored in localStorage
            </div>

            <div className="flex gap-2">
              <div className="flex-1 bg-[#242424] border-2 border-[#333] px-4 py-3 font-mono text-white flex items-center">
                {apiKeyVisible ? apiKey : maskApiKey(apiKey)}
              </div>

              <button
                onClick={() => setApiKeyVisible(!apiKeyVisible)}
                className="w-12 h-12 bg-[#242424] border-2 border-[#333] flex items-center justify-center text-[#A3A3A3] hover:text-white hover:border-[#D90018] transition-colors"
                aria-label={apiKeyVisible ? "Hide API key" : "Show API key"}
              >
                {apiKeyVisible ? <EyeOff size={16} /> : <Eye size={16} />}
              </button>

              <button
                onClick={() => copyText(apiKey)}
                className="w-12 h-12 bg-[#242424] border-2 border-[#333] flex items-center justify-center text-[#A3A3A3] hover:text-white hover:border-[#D90018] transition-colors"
                aria-label="Copy API key"
              >
                <Copy size={16} />
              </button>

              <button
                onClick={() => setRegenOpen(true)}
                className="w-12 h-12 bg-[#242424] border-2 border-[#333] flex items-center justify-center text-[#A3A3A3] hover:text-white hover:border-[#D90018] transition-colors"
                aria-label="Regenerate API key"
              >
                <RefreshCcw size={16} />
              </button>
            </div>
          </div>
        </P5Card>

        {/* Credits Card */}
        <P5Card
          title="CREDITS"
          headerAction={
            <button
              onClick={() => statusQuery.refetch()}
              disabled={statusQuery.isFetching}
              className="text-[#A3A3A3] hover:text-white transition-colors disabled:opacity-50"
              title="Refresh"
            >
              <RefreshCcw size={16} className={statusQuery.isFetching ? 'animate-spin' : ''} />
            </button>
          }
        >
          <div className="space-y-4">
            <div className="flex items-center justify-between text-sm font-mono">
              <span className="text-[#A3A3A3]">USAGE</span>
              <span className="text-white">{remaining} / {planCredits}</span>
            </div>

            <P5Progress value={remaining} max={planCredits} />

            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-3">
                <P5Badge variant={tier === "pro" ? "pro" : "default"}>
                  ◆ {tier.toUpperCase()}
                </P5Badge>
                <span className={`flex items-center gap-2 text-xs font-mono ${statusQuery.isFetching ? 'text-[#FFD700]' : 'text-[#00FF41]'}`}>
                  <span className={`w-2 h-2 rounded-full ${statusQuery.isFetching ? 'bg-[#FFD700]' : 'bg-[#00FF41]'}`} />
                  {statusQuery.isFetching ? "SYNCING..." : "SYNCED"}
                </span>
              </div>

              <P5Button onClick={() => setTopUpOpen(true)}>
                <Wallet size={14} />
                TOP UP WITH X402
              </P5Button>
            </div>
          </div>
        </P5Card>

        {/* Quick Start Card */}
        <P5Card title="QUICK START">
          <div className="space-y-4">
            <div className="text-xs text-[#555] font-mono">
              Claude Desktop Configuration
            </div>

            <div className="flex items-center gap-2 text-sm">
              <span className="text-[#A3A3A3] font-mono">API URL:</span>
              <span className="text-white font-mono">{apiUrl}</span>
            </div>

            <div className="bg-[#0A0A0A] border border-[#333] p-4 overflow-x-auto">
              <CodeBlock code={configJson} language="json" />
            </div>

            <div className="flex flex-wrap items-center justify-between gap-3">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={showRawConfig}
                  onChange={(e) => setShowRawConfig(e.target.checked)}
                  className="w-4 h-4 bg-[#242424] border-2 border-[#333] accent-[#D90018]"
                />
                <span className="text-xs text-[#A3A3A3] font-mono">SHOW RAW</span>
              </label>

              <P5Button onClick={() => copyText(configJson)}>
                <Copy size={14} />
                COPY CONFIG
              </P5Button>
            </div>
          </div>
        </P5Card>
      </div>

      {/* Regenerate Dialog */}
      <P5Dialog
        open={regenOpen}
        onClose={() => setRegenOpen(false)}
        title="REGENERATE API KEY"
        description="Generate a new API key string (client-side)."
        footer={
          <>
            <P5Button variant="secondary" onClick={() => setRegenOpen(false)}>
              CANCEL
            </P5Button>
            <P5Button onClick={regenerate}>
              REGENERATE
            </P5Button>
          </>
        }
      >
        <div className="text-sm text-[#A3A3A3] font-mono">
          This will generate a new random API key. The old key will no longer work.
        </div>
      </P5Dialog>

      {/* Top Up Dialog */}
      <P5Dialog
        open={topUpOpen}
        onClose={() => setTopUpOpen(false)}
        title="X402 TOP UP"
        description="Send CRO to the payment address. Credits are granted after backend verification."
        footer={
          <>
            <P5Button variant="secondary" onClick={() => setTopUpOpen(false)}>
              CLOSE
            </P5Button>
            <P5Button
              onClick={sendTopUpTransaction}
              disabled={
                !quote ||
                !isConnected ||
                !isCronos ||
                isSending ||
                !!paymentTxHash ||
                quoteQuery.isLoading
              }
            >
              {isSending
                ? "SENDING..."
                : paymentTxHash
                  ? "SENT"
                  : "SEND CRO"}
            </P5Button>
          </>
        }
      >
        <div className="space-y-3">
          {/* Step 1 */}
          <StepCard step={1} title="CONNECT WALLET">
            {isConnected && address ? (
              <>
                <div className="font-mono text-xs text-white break-all">{address}</div>
                <div className="mt-2 flex flex-wrap items-center gap-2">
                  <P5Badge variant={isCronos ? "success" : "warning"}>
                    {isCronos ? "CRONOS" : `CHAIN ${chainId ?? "UNKNOWN"}`}
                  </P5Badge>
                  {!isCronos && (
                    <P5Button
                      variant="secondary"
                      onClick={switchToCronos}
                      disabled={isSwitching}
                    >
                      SWITCH TO CRONOS
                    </P5Button>
                  )}
                </div>
              </>
            ) : (
              <span className="text-[#FFD700]">NOT CONNECTED</span>
            )}
          </StepCard>

          {/* Step 2 */}
          <StepCard step={2} title="SEND TRANSACTION">
            {quoteQuery.isLoading && (
              <span className="text-[#FFD700]">Loading quote...</span>
            )}
            {quoteQuery.isError && (
              <span className="text-[#FF4444]">{stringifyError(quoteQuery.error)}</span>
            )}
            {quote && (
              <div className="space-y-2">
                <div className="text-xs">
                  Amount: <span className="text-white">{quoteAmountCro}</span>
                  {" · "}
                  Credits: <span className="text-white">{quote.credits}</span>
                </div>
                <div className="flex gap-2">
                  <div className="flex-1 bg-[#242424] border border-[#333] px-3 py-2 font-mono text-xs text-white break-all">
                    {quote.payment_address}
                  </div>
                  <button
                    onClick={() => copyText(quote.payment_address)}
                    className="w-10 h-10 bg-[#242424] border border-[#333] flex items-center justify-center text-[#A3A3A3] hover:text-white hover:border-[#D90018] transition-colors"
                    title="Copy address"
                  >
                    <Copy size={14} />
                  </button>
                </div>
              </div>
            )}
          </StepCard>

          {/* Step 3 */}
          <StepCard step={3} title="WAIT FOR CONFIRMATION">
            <div className="flex items-center gap-2">
              <span>Status:</span>
              <P5Badge
                variant={
                  txLabel === "CONFIRMED"
                    ? "success"
                    : txLabel === "FAILED"
                      ? "destructive"
                      : "warning"
                }
              >
                {txLabel}
              </P5Badge>
            </div>
            {paymentTxHash ? (
              <div className="mt-2 flex gap-2">
                <div className="flex-1 bg-[#242424] border border-[#333] px-3 py-2 font-mono text-xs text-white break-all">
                  {paymentTxHash}
                </div>
                <button
                  onClick={() => copyText(paymentTxHash)}
                  className="w-10 h-10 bg-[#242424] border border-[#333] flex items-center justify-center text-[#A3A3A3] hover:text-white hover:border-[#D90018] transition-colors"
                  title="Copy tx hash"
                >
                  <Copy size={14} />
                </button>
              </div>
            ) : (
              <div className="mt-1 text-[#555]">Transaction not sent yet.</div>
            )}
            {isE2E && !paymentTxHash && (
              <div className="mt-2 flex flex-wrap gap-2">
                <P5Button
                  variant="secondary"
                  onClick={() =>
                    setPaymentTxHash(
                      "0x1111111111111111111111111111111111111111111111111111111111111111",
                    )
                  }
                >
                  MOCK TX (CREDITED)
                </P5Button>
                <P5Button
                  variant="secondary"
                  onClick={() =>
                    setPaymentTxHash(
                      "0x2222222222222222222222222222222222222222222222222222222222222222",
                    )
                  }
                >
                  MOCK TX (PENDING)
                </P5Button>
              </div>
            )}
          </StepCard>

          {/* Step 4 */}
          <StepCard step={4} title="BACKEND VERIFICATION">
            {verifyStatus ? (
              <span className="text-[#00FF41]">Status: {verifyStatus}</span>
            ) : paymentTxHash ? (
              <span className="text-[#FFD700]">Waiting for backend verification...</span>
            ) : (
              <span className="text-[#555]">Send the transaction first.</span>
            )}
            {verifyError && (
              <div className="mt-1 text-[#FF4444]" role="alert">
                {verifyError}
              </div>
            )}
          </StepCard>
        </div>
      </P5Dialog>
    </div>
  );
}

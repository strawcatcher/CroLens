import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { ToolName } from "@/types/api";

export type LogLevel = "info" | "error";

export interface SessionLogEntry {
  id: string;
  ts: number;
  level: LogLevel;
  tool: ToolName;
  message: string;
  traceId?: string;
}

export interface LatencySample {
  ts: number;
  tool: ToolName;
  latencyMs: number;
  status: "success" | "error";
}

interface AppState {
  apiKey: string;
  apiKeyVisible: boolean;
  setApiKey: (apiKey: string) => void;
  setApiKeyVisible: (visible: boolean) => void;

  tier: string;
  credits: number;
  planCredits: number;
  setBilling: (billing: {
    tier?: string;
    credits?: number;
    planCredits?: number;
  }) => void;

  logs: SessionLogEntry[];
  addLog: (
    entry: Omit<SessionLogEntry, "id" | "ts"> &
      Partial<Pick<SessionLogEntry, "id" | "ts">>,
  ) => void;
  clearLogs: () => void;

  latency: LatencySample[];
  addLatency: (sample: LatencySample) => void;
  clearLatency: () => void;
}

export function createDemoApiKey() {
  const raw =
    globalThis.crypto?.randomUUID?.() ??
    `${Date.now().toString(16)}${Math.random().toString(16).slice(2)}`;
  return `cl_sk_${raw.replaceAll("-", "")}`;
}

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      apiKey: createDemoApiKey(),
      apiKeyVisible: false,
      setApiKey: (apiKey) => {
        const trimmed = apiKey.trim();
        set({ apiKey: trimmed, tier: "free", credits: 50, planCredits: 50 });
      },
      setApiKeyVisible: (visible) => set({ apiKeyVisible: visible }),

      tier: "free",
      credits: 50,
      planCredits: 50,
      setBilling: (billing) =>
        set((state) => {
          const tier = billing.tier ?? state.tier;
          const credits = billing.credits ?? state.credits;
          const planCredits =
            billing.planCredits ??
            (tier === "free" ? Math.max(50, credits) : state.planCredits);
          return { tier, credits, planCredits: Math.max(planCredits, credits) };
        }),

      logs: [],
      addLog: (entry) => {
        const next: SessionLogEntry = {
          id:
            entry.id ??
            globalThis.crypto?.randomUUID?.() ??
            `${Date.now()}-${Math.random()}`,
          ts: entry.ts ?? Date.now(),
          level: entry.level,
          tool: entry.tool,
          message: entry.message,
          traceId: entry.traceId,
        };

        const current = get().logs;
        const capped =
          current.length >= 200 ? current.slice(current.length - 199) : current;
        set({ logs: [...capped, next] });
      },
      clearLogs: () => set({ logs: [] }),

      latency: [],
      addLatency: (sample) => {
        const current = get().latency;
        const capped =
          current.length >= 200 ? current.slice(current.length - 199) : current;
        set({ latency: [...capped, sample] });
      },
      clearLatency: () => set({ latency: [] }),
    }),
    {
      name: "crolens-web",
      version: 2,
      partialize: (state) => ({
        apiKey: state.apiKey,
        tier: state.tier,
        credits: state.credits,
        planCredits: state.planCredits,
      }),
    },
  ),
);

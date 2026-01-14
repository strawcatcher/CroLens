import { beforeEach, describe, expect, it } from "vitest";
import { createDemoApiKey, useAppStore } from "@/stores/app";

beforeEach(() => {
  localStorage.clear();
  useAppStore.setState({ logs: [], latency: [] });
});

describe("createDemoApiKey", () => {
  it("creates cl_sk_ keys without hyphens", () => {
    const key = createDemoApiKey();
    expect(key.startsWith("cl_sk_")).toBe(true);
    expect(key.includes("-")).toBe(false);
  });
});

describe("useAppStore", () => {
  it("setApiKey trims and resets billing defaults", () => {
    useAppStore.getState().setApiKey("  cl_sk_test  ");
    const state = useAppStore.getState();
    expect(state.apiKey).toBe("cl_sk_test");
    expect(state.tier).toBe("free");
    expect(state.credits).toBe(50);
    expect(state.planCredits).toBe(50);
  });

  it("setBilling enforces planCredits >= credits", () => {
    useAppStore.getState().setBilling({ tier: "free", credits: 10 });
    expect(useAppStore.getState().planCredits).toBe(50);

    useAppStore.getState().setBilling({ tier: "free", credits: 80 });
    expect(useAppStore.getState().planCredits).toBe(80);

    useAppStore.getState().setBilling({ tier: "pro", credits: 120 });
    expect(useAppStore.getState().planCredits).toBeGreaterThanOrEqual(120);
  });

  it("caps logs at 200 entries", () => {
    const addLog = useAppStore.getState().addLog;
    for (let i = 0; i < 250; i += 1) {
      addLog({ level: "info", tool: "get_account_summary", message: `m${i}` });
    }
    expect(useAppStore.getState().logs.length).toBe(200);
    expect(useAppStore.getState().logs[0]?.message).toBe("m50");
    expect(useAppStore.getState().logs[199]?.message).toBe("m249");
  });

  it("caps latency at 200 entries", () => {
    const addLatency = useAppStore.getState().addLatency;
    for (let i = 0; i < 250; i += 1) {
      addLatency({
        ts: i,
        tool: "get_account_summary",
        latencyMs: i,
        status: "success",
      });
    }
    expect(useAppStore.getState().latency.length).toBe(200);
    expect(useAppStore.getState().latency[0]?.latencyMs).toBe(50);
    expect(useAppStore.getState().latency[199]?.latencyMs).toBe(249);
  });
});


import { describe, expect, it, vi, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { AssetPieChart } from "./AssetPieChart";
import { DefiBarChart } from "./DefiBarChart";
import { TransactionFlowChart, createFlowStepsFromTx } from "./TransactionFlowChart";
import { StateChangeCard } from "./StateChangeCard";

// Mock ResizeObserver for Recharts ResponsiveContainer
vi.mock("recharts", async () => {
  const actual = await vi.importActual<typeof import("recharts")>("recharts");
  return {
    ...actual,
    ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
      <div data-testid="responsive-container" style={{ width: 400, height: 300 }}>
        {children}
      </div>
    ),
  };
});

afterEach(() => {
  cleanup();
});

describe("AssetPieChart", () => {
  it("renders empty state when no data", () => {
    render(<AssetPieChart data={[]} />);
    expect(screen.getByText("No assets to display")).toBeInTheDocument();
  });

  it("renders chart with data", () => {
    const data = [
      { symbol: "CRO", valueUsd: 1000 },
      { symbol: "WCRO", valueUsd: 500 },
    ];
    render(<AssetPieChart data={data} />);
    expect(screen.getByTestId("responsive-container")).toBeInTheDocument();
    expect(screen.getByText("$1,500")).toBeInTheDocument(); // Total
    expect(screen.getByText("TOTAL")).toBeInTheDocument();
    expect(screen.getByText("CRO")).toBeInTheDocument();
    expect(screen.getByText("WCRO")).toBeInTheDocument();
  });

  it("shows +N more when more than 4 assets", () => {
    const data = [
      { symbol: "CRO", valueUsd: 1000 },
      { symbol: "WCRO", valueUsd: 500 },
      { symbol: "USDC", valueUsd: 300 },
      { symbol: "ETH", valueUsd: 200 },
      { symbol: "BTC", valueUsd: 100 },
      { symbol: "VVS", valueUsd: 50 },
    ];
    render(<AssetPieChart data={data} />);
    expect(screen.getByText("+2 more")).toBeInTheDocument();
  });
});

describe("DefiBarChart", () => {
  it("renders empty state when no data", () => {
    render(<DefiBarChart data={[]} />);
    expect(screen.getByText("No DeFi positions")).toBeInTheDocument();
  });

  it("renders chart with protocol data", () => {
    const data = [
      { protocol: "VVS", valueUsd: 1000, type: "Liquidity" },
      { protocol: "Tectonic", valueUsd: 500, type: "Supply" },
    ];
    render(<DefiBarChart data={data} />);
    expect(screen.getByTestId("responsive-container")).toBeInTheDocument();
    expect(screen.getByText("PROTOCOL DISTRIBUTION")).toBeInTheDocument();
    expect(screen.getByText("$1,500")).toBeInTheDocument(); // Total
  });

  it("shows percentage for each protocol", () => {
    const data = [
      { protocol: "VVS", valueUsd: 750 },
      { protocol: "Tectonic", valueUsd: 250 },
    ];
    render(<DefiBarChart data={data} />);
    expect(screen.getByText("75.0%")).toBeInTheDocument();
    expect(screen.getByText("25.0%")).toBeInTheDocument();
  });
});

describe("TransactionFlowChart", () => {
  it("renders empty state when no steps", () => {
    render(<TransactionFlowChart steps={[]} />);
    expect(screen.getByText("No flow data available")).toBeInTheDocument();
  });

  it("renders flow steps", () => {
    const steps = [
      {
        from: { label: "USER", address: "0x1234567890123456789012345678901234567890", isUser: true },
        to: { label: "ROUTER", address: "0xabcdef1234567890123456789012345678901234", isContract: true },
        amount: "100",
        token: "CRO",
      },
    ];
    render(<TransactionFlowChart steps={steps} />);
    expect(screen.getByText("TRANSACTION FLOW")).toBeInTheDocument();
    expect(screen.getByText("USER")).toBeInTheDocument();
    expect(screen.getByText("ROUTER")).toBeInTheDocument();
    expect(screen.getByText("100")).toBeInTheDocument();
    expect(screen.getByText("CRO")).toBeInTheDocument();
  });

  it("shows legend", () => {
    const steps = [
      {
        from: { label: "USER", isUser: true },
        to: { label: "CONTRACT", isContract: true },
        amount: "1",
        token: "ETH",
      },
    ];
    render(<TransactionFlowChart steps={steps} />);
    expect(screen.getByText("User")).toBeInTheDocument();
    expect(screen.getByText("Contract")).toBeInTheDocument();
  });
});

describe("createFlowStepsFromTx", () => {
  it("creates swap flow steps", () => {
    const decoded = {
      from: "0xuser",
      to: "0xrouter",
      action: "Swap tokens",
      decoded: {
        method_name: "swapExactTokensForTokens",
        inputs: [
          { name: "amountIn", value: "100" },
          { name: "amountOutMin", value: "95" },
        ],
      },
    };
    const steps = createFlowStepsFromTx(decoded);
    expect(steps).toHaveLength(2);
    expect(steps[0].from.label).toBe("USER");
    expect(steps[0].to.label).toBe("ROUTER");
    expect(steps[0].amount).toBe("100");
    expect(steps[1].from.label).toBe("ROUTER");
    expect(steps[1].to.label).toBe("USER");
  });

  it("creates transfer flow steps", () => {
    const decoded = {
      from: "0xsender",
      to: "0xtoken",
      action: "Transfer tokens",
      decoded: {
        method_name: "transfer",
        inputs: [
          { name: "to", value: "0xrecipient" },
          { name: "amount", value: "50" },
        ],
      },
    };
    const steps = createFlowStepsFromTx(decoded);
    expect(steps).toHaveLength(1);
    expect(steps[0].from.label).toBe("FROM");
    expect(steps[0].to.address).toBe("0xrecipient");
    expect(steps[0].amount).toBe("50");
  });

  it("creates generic call flow steps", () => {
    const decoded = {
      from: "0xcaller",
      to: "0xcontract",
      action: "Call contract",
      decoded: {
        method_name: "doSomething",
      },
    };
    const steps = createFlowStepsFromTx(decoded);
    expect(steps).toHaveLength(1);
    expect(steps[0].from.label).toBe("CALLER");
    expect(steps[0].to.label).toBe("DOSOMETHING");
    expect(steps[0].token).toBe("CALL");
  });
});

describe("StateChangeCard", () => {
  it("renders empty state when no changes", () => {
    render(<StateChangeCard changes={[]} success={true} />);
    expect(screen.getByText("No state changes detected")).toBeInTheDocument();
  });

  it("renders success state with changes", () => {
    const changes = [
      {
        description: "Transfer out",
        from: "User wallet",
        to: "0xcontract",
        amount: "100",
        token: "CRO",
      },
    ];
    render(<StateChangeCard changes={changes} success={true} />);
    expect(screen.getByText("STATE CHANGES PREVIEW")).toBeInTheDocument();
    expect(screen.getByText("SIMULATION OK")).toBeInTheDocument();
    expect(screen.getByText("Transfer out")).toBeInTheDocument();
    expect(screen.getByText("-100")).toBeInTheDocument();
    expect(screen.getByText("CRO")).toBeInTheDocument();
  });

  it("renders failed state with warning", () => {
    const changes = [
      {
        description: "Failed transfer",
        from: "0xfrom",
        to: "0xto",
        amount: "50",
        token: "ETH",
      },
    ];
    render(<StateChangeCard changes={changes} success={false} />);
    expect(screen.getByText("SIMULATION FAILED")).toBeInTheDocument();
    expect(screen.getByText("WARNING")).toBeInTheDocument();
    expect(
      screen.getByText(
        "This transaction may fail or produce unexpected results. Review carefully before proceeding.",
      ),
    ).toBeInTheDocument();
  });

  it("shows gas estimate when provided", () => {
    const changes = [
      {
        description: "Test",
        from: "0x1",
        to: "0x2",
        amount: "1",
        token: "TEST",
      },
    ];
    render(<StateChangeCard changes={changes} success={true} gasEstimated="21000" />);
    expect(screen.getByText("Estimated Gas:")).toBeInTheDocument();
    expect(screen.getByText("21000")).toBeInTheDocument();
  });

  it("shows total changes count", () => {
    const changes = [
      { description: "Change 1", from: "a", to: "b", amount: "100", token: "T1" },
      { description: "Change 2", from: "c", to: "d", amount: "200", token: "T2" },
      { description: "Change 3", from: "e", to: "f", amount: "300", token: "T3" },
    ];
    render(<StateChangeCard changes={changes} success={true} />);
    expect(screen.getByText("Total changes")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
  });

  it("detects incoming transfers", () => {
    const changes = [
      {
        description: "Receive tokens",
        from: "0xcontract",
        to: "User wallet",
        amount: "100",
        token: "VVS",
      },
    ];
    render(<StateChangeCard changes={changes} success={true} />);
    // Should show + prefix for incoming
    expect(screen.getByText("+100")).toBeInTheDocument();
  });
});

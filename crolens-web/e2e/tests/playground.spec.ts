import { test, expect } from "@playwright/test";
import {
  STORAGE_KEY,
  TEST_KEYS,
  buildPersistedState,
} from "../fixtures/test-api-key";

test.beforeEach(async ({ page }) => {
  await page.route("**:8787/**", async (route) => {
    if (route.request().method() !== "POST") {
      await route.continue();
      return;
    }

    let body: any;
    try {
      body = route.request().postDataJSON();
    } catch {
      await route.continue();
      return;
    }

    if (!body || body.method !== "tools/call") {
      await route.continue();
      return;
    }

    const tool = body?.params?.name;
    const id = body?.id ?? 1;
    const now = Date.now();
    const meta = { trace_id: "e2e", timestamp: now, latency_ms: 1, cached: false };

    function ok(result: unknown) {
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ jsonrpc: "2.0", id, result }),
      });
    }

    switch (tool) {
      case "search_contract":
        return ok({
          results: [
            {
              name: "VVS Router",
              address: "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae",
              type: "DEX Router",
              protocol: "vvs",
            },
          ],
          meta,
        });
      case "get_account_summary": {
        const address = body?.params?.arguments?.address ?? "0x0000000000000000000000000000000000000000";
        return ok({
          address,
          total_net_worth_usd: "150.00",
          wallet: [
            {
              token_address: "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23",
              symbol: "WCRO",
              decimals: 18,
              balance: "1000000000000000000",
              balance_formatted: "1.0",
              price_usd: "0.10",
              value_usd: "0.10",
            },
            {
              token_address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59",
              symbol: "USDC",
              decimals: 6,
              balance: "1000000",
              balance_formatted: "1.0",
              price_usd: "1.00",
              value_usd: "1.00",
            },
          ],
          defi_summary: {
            total_defi_value_usd: "148.90",
            vvs_liquidity_usd: "100.00",
            tectonic_supply_usd: "50.00",
            tectonic_borrow_usd: "1.10",
          },
          meta,
        });
      }
      case "get_defi_positions": {
        const address = body?.params?.arguments?.address ?? "0x0000000000000000000000000000000000000000";
        return ok({
          address,
          vvs: {
            total_liquidity_usd: "100.00",
            total_pending_rewards_usd: "0.00",
            positions: [],
          },
          tectonic: {
            total_supply_usd: "50.00",
            total_borrow_usd: "0.00",
            net_value_usd: "50.00",
            health_factor: "∞",
            supplies: [],
            borrows: [],
          },
          meta,
        });
      }
      case "decode_transaction": {
        const txHash = body?.params?.arguments?.tx_hash ?? "0x";
        return ok({
          hash: txHash,
          from: "0x2222222222222222222222222222222222222222",
          to: "0x1111111111111111111111111111111111111111",
          action: "Transfer",
          protocol: null,
          status: "0x1",
          gas_used: "21000",
          decoded: { method_name: "transfer", params: { to: "0x0", amount: "0" } },
          meta,
        });
      }
      case "simulate_transaction": {
        return ok({
          success: true,
          simulation_available: true,
          gas_estimated: "21000",
          state_changes: [],
          risk_assessment: { level: "low", warnings: [] },
          meta,
        });
      }
      default:
        await route.continue();
        return;
    }
  });

  await page.addInitScript(
    ({ key, value }) => {
      localStorage.clear();
      localStorage.setItem(key, JSON.stringify(value));
    },
    {
      key: STORAGE_KEY,
      value: buildPersistedState({
        apiKey: TEST_KEYS.pro,
        tier: "pro",
        credits: 1000,
        planCredits: 1000,
      }),
    },
  );
});

test("selects search_contract and shows query input", async ({ page }) => {
  await page.goto("/playground");
  await page.getByRole("tab", { name: "search_contract" }).click();
  await expect(page.getByLabel("Query")).toBeVisible();
});

test("selects get_account_summary and shows address input", async ({ page }) => {
  await page.goto("/playground");
  await page.getByRole("tab", { name: "get_account_summary" }).click();
  await expect(page.getByRole("textbox", { name: "TARGET ADDRESS" })).toBeVisible();
});

test("shows invalid address validation and disables Execute", async ({ page }) => {
  await page.goto("/playground");
  await page.getByRole("tab", { name: "get_account_summary" }).click();

  await page.getByRole("textbox", { name: "TARGET ADDRESS" }).fill("0xabc");
  await expect(page.getByText("Invalid address")).toBeVisible();
  await expect(page.getByRole("button", { name: /EXECUTE/i })).toBeDisabled();
});

test("executes search_contract and toggles raw JSON", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "search_contract" }).click();
  await page.getByLabel("Query").fill("VVS");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("RESULTS")).toBeVisible();
  await expect(page.getByText("VVS Router")).toBeVisible();

  await page.getByRole("button", { name: "[RAW]" }).click();
  await expect(page.getByText('"jsonrpc": "2.0"')).toBeVisible();
});

test("get_account_summary shows asset pie chart", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_account_summary" }).click();
  await page
    .getByRole("textbox", { name: "TARGET ADDRESS" })
    .fill("0x6f3dE5468D8de8DD3DB9DB02cc72ae59a50D603C");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  // Check pie chart header
  await expect(page.getByText("ASSET DISTRIBUTION")).toBeVisible();
  // Check TOTAL label in center of pie chart
  await expect(page.getByText("TOTAL")).toBeVisible();
});

test("get_defi_positions shows DeFi bar chart", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_defi_positions" }).click();
  await page
    .getByRole("textbox", { name: "TARGET ADDRESS" })
    .fill("0x6f3dE5468D8de8DD3DB9DB02cc72ae59a50D603C");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  // Check bar chart header
  await expect(page.getByText("PROTOCOL DISTRIBUTION")).toBeVisible();
});

test("decode_transaction shows transaction flow chart", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "decode_transaction" }).click();
  await page
    .getByRole("textbox", { name: "TX HASH" })
    .fill("0x1234567890123456789012345678901234567890123456789012345678901234");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("DECODED TRANSACTION")).toBeVisible();
});

test("simulate_transaction shows state changes card", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "simulate_transaction" }).click();
  await page
    .getByRole("textbox", { name: "TARGET ADDRESS" })
    .fill("0x6f3dE5468D8de8DD3DB9DB02cc72ae59a50D603C");
  await page
    .getByRole("textbox", { name: "TO" })
    .fill("0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae");
  await page.locator('textarea[placeholder="0x..."]').fill("0x");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  // Check simulation header
  await expect(page.getByText("SIMULATION")).toBeVisible();
});

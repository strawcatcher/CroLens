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
      case "get_token_info": {
        const token = body?.params?.arguments?.token ?? "UNKNOWN";
        return ok({
          address: "0x2D03bece6747ADC00E1a131BBA1469C15fD11e03",
          name: `${token} Token`,
          symbol: token,
          decimals: 18,
          total_supply: "100000000",
          price_usd: "0.12",
          market_cap_usd: "123456.78",
          pools: [
            {
              dex: "vvs",
              pair: `${token}-WCRO`,
              pair_address: "0x814920d1b8007207db6cb5a2dd92bf0b082bdba1",
              tvl_usd: "2000000.00",
            },
          ],
          meta,
        });
      }
      case "get_pool_info": {
        return ok({
          pool_address: "0x814920d1b8007207db6cb5a2dd92bf0b082bdba1",
          dex: "vvs",
          token0: {
            address: "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23",
            symbol: "WCRO",
            reserve: "1000000",
          },
          token1: {
            address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59",
            symbol: "USDC",
            reserve: "1000000",
          },
          total_supply: "100000",
          tvl_usd: "2000000.00",
          apy: "12.34",
          meta,
        });
      }
      case "get_gas_price": {
        return ok({
          gas_price_gwei: "5000.00",
          gas_price_wei: "5000000000000",
          level: "medium",
          estimates: [
            {
              operation: "transfer",
              gas_units: 21000,
              cost_cro: "0.105000",
              cost_usd: "0.0105",
            },
            {
              operation: "swap",
              gas_units: 150000,
              cost_cro: "0.750000",
              cost_usd: "0.0750",
            },
          ],
          recommendation: "Gas prices are moderate. Normal operations recommended.",
          meta,
        });
      }
      case "get_approval_status": {
        const address =
          body?.params?.arguments?.address ??
          "0x0000000000000000000000000000000000000000";
        return ok({
          address,
          approvals: [
            {
              token_symbol: "USDC",
              token_address: "0xc21223249CA28397B4B6541dfFaEcC539BfF0c59",
              spender_address: "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae",
              spender_name: "VVS Router",
              protocol: "VVS Finance",
              allowance: "unlimited",
              is_unlimited: true,
              risk_level: "warning",
            },
          ],
          summary: {
            total_approvals: 1,
            unlimited_approvals: 1,
            risk_score: 20,
          },
          meta,
        });
      }
      case "get_block_info": {
        return ok({
          number: 15000000,
          hash: "0xabc1230000000000000000000000000000000000000000000000000000000000",
          timestamp: Math.floor(Date.now() / 1000),
          timestamp_relative: "just now",
          transactions_count: 150,
          gas_used: "8000000",
          gas_limit: "15000000",
          gas_used_percent: "53.33",
          base_fee_gwei: "5000.00",
          miner: "0x0000000000000000000000000000000000000000",
          meta,
        });
      }
      case "get_vvs_farms": {
        return ok({
          farms: [
            {
              pool_id: 0,
              lp_symbol: "CRO-USDC",
              tvl_usd: "1000000.00",
              apy: "12.5",
            },
            {
              pool_id: 1,
              lp_symbol: "CRO-VVS",
              tvl_usd: "500000.00",
              apy: "25.0",
            },
          ],
          total_tvl_usd: "1500000.00",
          meta,
        });
      }
      case "get_tectonic_markets": {
        return ok({
          markets: [
            {
              symbol: "USDC",
              supply_apy: "3.5",
              borrow_apy: "5.2",
              total_supply_usd: "10000000.00",
            },
            {
              symbol: "CRO",
              supply_apy: "2.1",
              borrow_apy: "4.8",
              total_supply_usd: "5000000.00",
            },
          ],
          total_supply_usd: "15000000.00",
          meta,
        });
      }
      case "get_cro_overview": {
        return ok({
          chain_id: 25,
          block_number: "0xe4e1c0",
          price_usd: "0.10",
          meta,
        });
      }
      case "get_protocol_stats": {
        return ok({
          protocols: [
            { name: "VVS Finance", tvl_usd: "50000000.00", pools_count: 15 },
            { name: "Tectonic", tvl_usd: "30000000.00", markets_count: 8 },
          ],
          total_tvl_usd: "80000000.00",
          meta,
        });
      }
      case "get_health_alerts": {
        const address =
          body?.params?.arguments?.address ??
          "0x0000000000000000000000000000000000000000";
        return ok({
          address,
          alerts: [
            {
              category: "approvals",
              level: "warning",
              message: "High token approval risk score.",
            },
          ],
          meta,
        });
      }
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

test("get_token_info shows token details", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_token_info" }).click();
  await page.getByRole("textbox", { name: "TOKEN" }).fill("VVS");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("TOKEN INFO")).toBeVisible();
  await expect(page.getByText("VVS Token")).toBeVisible();
});

test("get_pool_info shows pool details", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_pool_info" }).click();
  await page.getByRole("textbox", { name: "POOL" }).fill("CRO-USDC");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("POOL INFO")).toBeVisible();
  await expect(page.getByText("WCRO / USDC")).toBeVisible();
});

test("get_gas_price shows gas costs", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_gas_price" }).click();
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("GAS PRICE", { exact: true })).toBeVisible();
  await expect(page.getByText("5000.00")).toBeVisible();
});

test("get_approval_status shows risk badge", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_approval_status" }).click();
  await page
    .getByRole("textbox", { name: "TARGET ADDRESS" })
    .fill("0x6f3dE5468D8de8DD3DB9DB02cc72ae59a50D603C");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("TOKEN APPROVALS")).toBeVisible();
  await expect(page.getByText("WARNING")).toBeVisible();
});

test("get_block_info shows block number", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_block_info" }).click();
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("BLOCK INFORMATION")).toBeVisible();
  await expect(page.getByText("#15,000,000")).toBeVisible();
});

test("get_vvs_farms shows farm list", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_vvs_farms" }).click();
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("CRO-USDC")).toBeVisible();
});

test("get_tectonic_markets shows market list", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_tectonic_markets" }).click();
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("USDC")).toBeVisible();
});

test("get_cro_overview shows chain info", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_cro_overview" }).click();
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("block_number")).toBeVisible();
});

test("get_protocol_stats shows protocol names", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_protocol_stats" }).click();
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("VVS Finance")).toBeVisible();
});

test("get_health_alerts shows warnings", async ({ page }) => {
  await page.goto("/playground");

  await page.getByRole("tab", { name: "get_health_alerts" }).click();
  await page
    .getByRole("textbox", { name: "TARGET ADDRESS" })
    .fill("0x6f3dE5468D8de8DD3DB9DB02cc72ae59a50D603C");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  await expect(page.getByText("High token approval risk score.")).toBeVisible();
});

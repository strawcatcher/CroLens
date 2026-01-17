import { test, expect, type Page } from "@playwright/test";
import {
  STORAGE_KEY,
  TEST_KEYS,
  buildPersistedState,
} from "../fixtures/test-api-key";

async function setApiKey(page: Page, apiKey: string, tier: string) {
  await page.addInitScript(
    ({ key, value }) => {
      localStorage.clear();
      localStorage.setItem(key, JSON.stringify(value));
    },
    {
      key: STORAGE_KEY,
      value: buildPersistedState({
        apiKey,
        tier,
        credits: tier === "pro" ? 1000 : 50,
        planCredits: tier === "pro" ? 1000 : 50,
      }),
    },
  );
}

test("shows payment required message (-32002)", async ({ page }) => {
  await setApiKey(page, TEST_KEYS.freeZero, "free");
  await page.goto("/playground");

  await page.getByRole("tab", { name: "search_contract" }).click();
  await page.getByLabel("Query").fill("VVS");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  const alert = page.getByRole("alert");
  await expect(alert).toContainText(
    "Payment required: insufficient credits (top up via x402).",
  );
});

test("shows rate limit message (-32003)", async ({ page }) => {
  await setApiKey(page, TEST_KEYS.pro, "pro");

  await page.route("**:8787/**", async (route) => {
    if (route.request().method() !== "POST") {
      await route.continue();
      return;
    }
    await route.fulfill({
      status: 429,
      contentType: "application/json",
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        error: { code: -32003, message: "Rate limit exceeded" },
      }),
    });
  });

  await page.goto("/playground");
  await page.getByRole("tab", { name: "search_contract" }).click();
  await page.getByLabel("Query").fill("VVS");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  const alert = page.getByRole("alert");
  await expect(alert).toContainText("Rate limit exceeded. Please retry later.");
});

test("shows RPC error message (-32500)", async ({ page }) => {
  await setApiKey(page, TEST_KEYS.pro, "pro");

  await page.route("**:8787/**", async (route) => {
    if (route.request().method() !== "POST") {
      await route.continue();
      return;
    }
    await route.fulfill({
      status: 500,
      contentType: "application/json",
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        error: { code: -32500, message: "RPC error" },
      }),
    });
  });

  await page.goto("/playground");
  await page.getByRole("tab", { name: "search_contract" }).click();
  await page.getByLabel("Query").fill("VVS");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  const alert = page.getByRole("alert");
  await expect(alert).toContainText("RPC error. Please retry.");
});

test("shows network error message on fetch failure", async ({ page }) => {
  await setApiKey(page, TEST_KEYS.pro, "pro");

  await page.route("**:8787/**", async (route) => {
    if (route.request().method() !== "POST") {
      await route.continue();
      return;
    }
    await route.abort();
  });

  await page.goto("/playground");
  await page.getByRole("tab", { name: "search_contract" }).click();
  await page.getByLabel("Query").fill("VVS");
  await page.getByRole("button", { name: /EXECUTE/i }).click();

  const alert = page.getByRole("alert");
  await expect(alert).toContainText("Network error. Please retry.");
});

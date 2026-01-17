import { test, expect } from "@playwright/test";
import {
  STORAGE_KEY,
  TEST_KEYS,
  TEST_TX,
  buildPersistedState,
} from "../fixtures/test-api-key";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(
    ({ key, value }) => {
      localStorage.clear();
      localStorage.setItem(key, JSON.stringify(value));
    },
    {
      key: STORAGE_KEY,
      value: buildPersistedState({
        apiKey: TEST_KEYS.topup,
        tier: "free",
        credits: 50,
        planCredits: 50,
      }),
    },
  );
});

test("top up flow credits the API key in E2E mode", async ({ page }) => {
  await page.goto("/console");

  await expect(page.getByText(/PLAN:\s*FREE/i).first()).toBeVisible();

  await page.getByRole("button", { name: /TOP UP WITH X402/i }).click();

  await expect(page.getByText(/X402 TOP UP/i)).toBeVisible();
  await expect(
    page.getByText("0x1111111111111111111111111111111111111111").first(),
  ).toBeVisible();

  await page.getByRole("button", { name: /MOCK TX \(CREDITED\)/i }).click();
  await expect(page.getByText(TEST_TX.credited)).toBeVisible();

  await expect(page.getByText("Status: credited")).toBeVisible();
  await expect(page.getByText(/PLAN:\s*PRO/i).first()).toBeVisible();
  await expect(page.getByText(/1050\s*\/\s*1050/)).toBeVisible();
});

test("loads quote details in the top up dialog", async ({ page }) => {
  await page.goto("/console");
  await page.getByRole("button", { name: /TOP UP WITH X402/i }).click();

  await expect(page.getByText(/X402 TOP UP/i)).toBeVisible();
  await expect(page.getByText(/Credits:\s*1000/)).toBeVisible();
  await expect(
    page.getByText("0x1111111111111111111111111111111111111111").first(),
  ).toBeVisible();
});

test("disables Send CRO button when wallet is not connected", async ({ page }) => {
  await page.goto("/console");
  await page.getByRole("button", { name: /TOP UP WITH X402/i }).click();

  const send = page.getByRole("button", { name: /SEND CRO/i });
  await expect(send).toBeDisabled();
});

test("shows pending verification status for pending tx", async ({ page }) => {
  await page.goto("/console");
  await page.getByRole("button", { name: /TOP UP WITH X402/i }).click();

  await page.getByRole("button", { name: /MOCK TX \(PENDING\)/i }).click();
  await expect(page.getByText("Status: pending")).toBeVisible();
});

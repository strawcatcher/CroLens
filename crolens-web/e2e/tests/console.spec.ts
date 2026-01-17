import { test, expect } from "@playwright/test";
import {
  STORAGE_KEY,
  TEST_KEYS,
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
        apiKey: TEST_KEYS.free,
        tier: "free",
        credits: 100,
        planCredits: 100,
      }),
    },
  );
});

test("shows API key and credits", async ({ page }) => {
  await page.goto("/console");

  await page.getByRole("button", { name: "Show API key" }).click();
  const apiKeyRow = page.getByRole("button", { name: "Hide API key" }).locator("..");
  await expect(apiKeyRow.getByText(TEST_KEYS.free, { exact: true })).toBeVisible();

  await expect(page.getByText(/100\s*\/\s*100/)).toBeVisible();
});

test("copies API key to clipboard", async ({ page }) => {
  await page.goto("/console");

  await page.getByRole("button", { name: "Copy API key" }).click();
  const clipboard = await page.evaluate(() => navigator.clipboard.readText());
  expect(clipboard).toBe(TEST_KEYS.free);
});

test("copies Claude Desktop config to clipboard", async ({ page }) => {
  await page.goto("/console");

  await page.getByRole("button", { name: "Copy Config" }).click();
  const clipboard = await page.evaluate(() => navigator.clipboard.readText());
  expect(clipboard).toContain('"CROLENS_API_KEY"');
  expect(clipboard).toContain(TEST_KEYS.free);
  expect(clipboard).toContain('"CROLENS_API_URL"');
});

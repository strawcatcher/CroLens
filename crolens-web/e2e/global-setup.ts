import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const webDir = path.resolve(__dirname, "..");
const apiDir = path.resolve(webDir, "../crolens-api");

const webPort = 4173;
const apiUrl = "http://127.0.0.1:8787";
const webUrl = `http://127.0.0.1:${webPort}`;

const pidsFile = path.join(__dirname, ".pids.json");

async function waitForUrl(url: string, timeoutMs: number) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    try {
      const res = await fetch(url);
      if (res.ok) return;
    } catch {
      // Ignore until timeout.
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`Timed out waiting for ${url}`);
}

export default async function globalSetup() {
  execFileSync(path.join(apiDir, "tests/integration/setup.sh"), {
    cwd: apiDir,
    stdio: "inherit",
  });

  const vite = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(webPort), "--strictPort"],
    {
      cwd: webDir,
      env: {
        ...process.env,
        VITE_API_URL: apiUrl,
        VITE_E2E: "true",
      },
      stdio: "inherit",
    },
  );

  if (!vite.pid) {
    throw new Error("Failed to start Vite dev server (missing pid).");
  }

  fs.writeFileSync(
    pidsFile,
    JSON.stringify({ vitePid: vite.pid, apiUrl, webUrl }, null, 2),
    "utf8",
  );

  await waitForUrl(webUrl, 60_000);
}


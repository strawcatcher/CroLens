import { execFileSync } from "node:child_process";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const webDir = path.resolve(__dirname, "..");
const apiDir = path.resolve(webDir, "../crolens-api");

const pidsFile = path.join(__dirname, ".pids.json");

function killPid(pid: number, name: string) {
  try {
    process.kill(pid, "SIGTERM");
  } catch {
    return;
  }

  const started = Date.now();
  while (Date.now() - started < 5_000) {
    try {
      process.kill(pid, 0);
      // still alive
    } catch {
      return;
    }
  }

  try {
    process.kill(pid, "SIGKILL");
  } catch {
    // ignore
  }
}

export default async function globalTeardown() {
  if (fs.existsSync(pidsFile)) {
    try {
      const raw = fs.readFileSync(pidsFile, "utf8");
      const data = JSON.parse(raw) as { vitePid?: number };
      if (typeof data.vitePid === "number") {
        killPid(data.vitePid, "vite");
      }
    } catch {
      // ignore
    }
    fs.rmSync(pidsFile, { force: true });
  }

  execFileSync(path.join(apiDir, "tests/integration/teardown.sh"), {
    cwd: apiDir,
    stdio: "inherit",
  });
}


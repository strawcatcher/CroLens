import http from "node:http";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

function parseArgs(argv) {
  const out = { port: 19000, fixtures: null };
  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--port") {
      out.port = Number(argv[i + 1] ?? "19000");
      i += 1;
      continue;
    }
    if (arg === "--fixtures") {
      out.fixtures = argv[i + 1] ?? null;
      i += 1;
      continue;
    }
  }
  return out;
}

function readJsonBody(req) {
  return new Promise((resolvePromise, rejectPromise) => {
    let raw = "";
    req.setEncoding("utf8");
    req.on("data", (chunk) => {
      raw += chunk;
    });
    req.on("end", () => {
      if (raw.trim().length === 0) {
        resolvePromise(null);
        return;
      }
      try {
        resolvePromise(JSON.parse(raw));
      } catch (err) {
        rejectPromise(err);
      }
    });
    req.on("error", rejectPromise);
  });
}

function json(res, status, payload) {
  const body = JSON.stringify(payload);
  res.writeHead(status, {
    "Content-Type": "application/json",
    "Content-Length": Buffer.byteLength(body),
  });
  res.end(body);
}

const args = parseArgs(process.argv);
const fixturesPath = args.fixtures
  ? resolve(process.cwd(), args.fixtures)
  : resolve(process.cwd(), "tests/integration/fixtures/rpc_responses.json");

let fixtures;
try {
  fixtures = JSON.parse(readFileSync(fixturesPath, "utf8"));
} catch (err) {
  console.error(`[mock-rpc] Failed to load fixtures: ${fixturesPath}`, err);
  process.exit(1);
}

let mode = "up";

function lookupResponse(method, params) {
  const entry = fixtures[method];
  if (!entry) return null;

  if (
    entry &&
    typeof entry === "object" &&
    !Array.isArray(entry) &&
    Object.prototype.hasOwnProperty.call(entry, "__default")
  ) {
    const key = Array.isArray(params) ? String(params[0] ?? "") : "";
    return entry[key] ?? entry.__default ?? null;
  }

  return entry;
}

const server = http.createServer(async (req, res) => {
  try {
    if (!req.url) {
      json(res, 404, { error: "Missing url" });
      return;
    }

    if (req.url === "/__health") {
      json(res, 200, { ok: true, mode });
      return;
    }

    if (req.url === "/__mode") {
      if (req.method !== "POST") {
        json(res, 405, { error: "Method not allowed" });
        return;
      }
      const body = await readJsonBody(req);
      const next = body && typeof body === "object" ? body.mode : null;
      if (next !== "up" && next !== "down") {
        json(res, 400, { ok: false, error: "mode must be 'up' or 'down'" });
        return;
      }
      mode = next;
      json(res, 200, { ok: true, mode });
      return;
    }

    if (req.method !== "POST") {
      json(res, 405, { error: "Method not allowed" });
      return;
    }

    const body = await readJsonBody(req);
    const id = body && typeof body === "object" && "id" in body ? body.id : 1;
    const method = body && typeof body === "object" ? body.method : null;
    const params = body && typeof body === "object" ? body.params : [];

    if (mode === "down") {
      json(res, 200, {
        jsonrpc: "2.0",
        id,
        error: { code: -32000, message: "Mock RPC is down" },
      });
      return;
    }

    if (typeof method !== "string" || method.trim().length === 0) {
      json(res, 200, {
        jsonrpc: "2.0",
        id,
        error: { code: -32600, message: "Invalid request" },
      });
      return;
    }

    const resp = lookupResponse(method, params);
    if (!resp) {
      json(res, 200, {
        jsonrpc: "2.0",
        id,
        error: { code: -32601, message: `Method not mocked: ${method}` },
      });
      return;
    }

    json(res, 200, { jsonrpc: "2.0", id, ...resp });
  } catch (err) {
    console.error("[mock-rpc] Unhandled error:", err);
    json(res, 500, { error: String(err) });
  }
});

server.listen(args.port, "127.0.0.1", () => {
  console.log(`[mock-rpc] Listening on http://127.0.0.1:${args.port}`);
  console.log(`[mock-rpc] Fixtures: ${fixturesPath}`);
});


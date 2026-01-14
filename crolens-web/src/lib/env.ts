export function validateEnv() {
  const required = ["VITE_API_URL"] as const;
  const env = import.meta.env as unknown as Record<string, unknown>;

  const missing = required.filter((key) => {
    const value = env[key];
    return typeof value !== "string" || value.trim().length === 0;
  });

  if (missing.length === 0) return;

  if (import.meta.env.PROD) {
    throw new Error(`Missing env vars: ${missing.join(", ")}`);
  }

  console.warn(
    `[env] Missing env vars: ${missing.join(", ")} (dev uses defaults)`,
  );
}


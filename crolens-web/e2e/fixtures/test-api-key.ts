export const STORAGE_KEY = "crolens-web";

export const TEST_KEYS = {
  free: "cl_sk_test_free_001",
  freeZero: "cl_sk_test_free_zero",
  pro: "cl_sk_test_pro_001",
  topup: "cl_sk_test_free_topup",
} as const;

export const TEST_TX = {
  credited:
    "0x1111111111111111111111111111111111111111111111111111111111111111",
  pending:
    "0x2222222222222222222222222222222222222222222222222222222222222222",
} as const;

export function buildPersistedState(input: {
  apiKey: string;
  tier?: string;
  credits?: number;
  planCredits?: number;
}) {
  const tier = input.tier ?? "free";
  const credits = input.credits ?? 50;
  const planCredits = input.planCredits ?? credits;

  return {
    state: {
      apiKey: input.apiKey,
      tier,
      credits,
      planCredits,
    },
    version: 2,
  };
}

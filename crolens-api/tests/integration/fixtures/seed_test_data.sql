-- Test fixtures for local D1 (used by tests/integration/setup.sh).

DELETE FROM payments;
DELETE FROM api_keys;
-- Keep tokens empty so get_account_summary can run in integration tests without mocking eth_call/multicall.
DELETE FROM tokens;

INSERT INTO api_keys (api_key, tier, credits, daily_used) VALUES
  ('cl_sk_test_free_001', 'free', 100, 0),
  ('cl_sk_test_free_rl', 'free', 100, 0),
  ('cl_sk_test_free_zero', 'free', 0, 0),
  ('cl_sk_test_pro_001', 'pro', 1000, 0),
  ('cl_sk_test_free_topup', 'free', 50, 0)
ON CONFLICT(api_key) DO UPDATE SET
  tier = excluded.tier,
  credits = excluded.credits,
  daily_used = 0;

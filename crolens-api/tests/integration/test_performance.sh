#!/usr/bin/env bash
set -euo pipefail

# Performance benchmark tests for CroLens API
# Measures latency and throughput for each MCP tool

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./lib.sh
source "${INTEGRATION_DIR}/lib.sh"

load_pids

# Number of iterations for each test
ITERATIONS="${PERF_ITERATIONS:-10}"
# Target latency thresholds (ms)
THRESHOLD_SEARCH=100
THRESHOLD_SIMULATE=500
THRESHOLD_DEFI=600
THRESHOLD_DECODE=700
THRESHOLD_ACCOUNT=1000
THRESHOLD_SWAP=1500

# Test addresses
TEST_ADDRESS="0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23"
TEST_TX_HASH="${TEST_TX_VALID:-}"

echo "============================================"
echo "Performance Benchmark Tests"
echo "Iterations per test: ${ITERATIONS}"
echo "============================================"
echo ""

# Measure latency for a single request
measure_latency() {
    local name="$1"
    local payload="$2"

    local start end elapsed
    start=$(date +%s%3N)
    http_post_json "${BASE_URL}/" "${payload}" -H "x-api-key: ${TEST_PRO_KEY}" >/dev/null 2>&1
    end=$(date +%s%3N)
    elapsed=$((end - start))
    echo "${elapsed}"
}

# Run benchmark for a tool
run_benchmark() {
    local name="$1"
    local payload="$2"
    local threshold="$3"

    echo -n "[perf] ${name}: "

    local total=0
    local min=999999
    local max=0
    local latencies=()

    for ((i=1; i<=ITERATIONS; i++)); do
        local lat
        lat=$(measure_latency "${name}" "${payload}")
        latencies+=("${lat}")
        total=$((total + lat))
        ((lat < min)) && min=${lat}
        ((lat > max)) && max=${lat}
    done

    local avg=$((total / ITERATIONS))

    # Calculate p50, p95, p99 (simple implementation)
    IFS=$'\n' sorted=($(sort -n <<<"${latencies[*]}"))
    unset IFS
    local p50_idx=$((ITERATIONS / 2))
    local p95_idx=$((ITERATIONS * 95 / 100))
    local p99_idx=$((ITERATIONS * 99 / 100))

    # Ensure indices are within bounds
    ((p50_idx >= ITERATIONS)) && p50_idx=$((ITERATIONS - 1))
    ((p95_idx >= ITERATIONS)) && p95_idx=$((ITERATIONS - 1))
    ((p99_idx >= ITERATIONS)) && p99_idx=$((ITERATIONS - 1))

    local p50=${sorted[${p50_idx}]}
    local p95=${sorted[${p95_idx}]}
    local p99=${sorted[${p99_idx}]}

    echo "avg=${avg}ms min=${min}ms max=${max}ms p50=${p50}ms p95=${p95}ms p99=${p99}ms"

    # Check against threshold
    if ((avg > threshold)); then
        echo "  WARNING: avg ${avg}ms exceeds threshold ${threshold}ms"
    fi

    # Return avg for summary
    echo "${avg}" > "/tmp/perf_${name}.txt"
}

# Benchmark: search_contract (fastest, DB only)
run_benchmark "search_contract" \
    '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_contract","arguments":{"query":"VVS","limit":5}}}' \
    "${THRESHOLD_SEARCH}"

# Benchmark: simulate_transaction (1 RPC call)
run_benchmark "simulate_transaction" \
    '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"simulate_transaction","arguments":{"from":"0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23","to":"0xc21223249CA28397B4B6541dfFaEcC539BfF0c59","data":"0xa9059cbb0000000000000000000000001234567890123456789012345678901234567890000000000000000000000000000000000000000000000000000000000000000a","value":"0","simple_mode":true}}}' \
    "${THRESHOLD_SIMULATE}"

# Benchmark: get_defi_positions (batch RPC)
run_benchmark "get_defi_positions" \
    "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"get_defi_positions\",\"arguments\":{\"address\":\"${TEST_ADDRESS}\",\"simple_mode\":true}}}" \
    "${THRESHOLD_DEFI}"

# Benchmark: decode_transaction (if we have a tx hash)
if [[ -n "${TEST_TX_HASH}" ]]; then
    run_benchmark "decode_transaction" \
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"decode_transaction\",\"arguments\":{\"tx_hash\":\"${TEST_TX_HASH}\",\"simple_mode\":true}}}" \
        "${THRESHOLD_DECODE}"
else
    echo "[perf] decode_transaction: SKIPPED (no TEST_TX_VALID set)"
fi

# Benchmark: get_account_summary (multiple RPC + price)
run_benchmark "get_account_summary" \
    "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"get_account_summary\",\"arguments\":{\"address\":\"${TEST_ADDRESS}\",\"simple_mode\":true}}}" \
    "${THRESHOLD_ACCOUNT}"

# Benchmark: construct_swap_tx (multiple RPC + simulation)
run_benchmark "construct_swap_tx" \
    '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"construct_swap_tx","arguments":{"from":"0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23","token_in":"USDC","token_out":"VVS","amount_in":"1000000","slippage_bps":100}}}' \
    "${THRESHOLD_SWAP}"

# Summary
echo ""
echo "============================================"
echo "Performance Summary (avg latency)"
echo "============================================"
echo "  search_contract:      $(cat /tmp/perf_search_contract.txt 2>/dev/null || echo 'N/A')ms (target: <${THRESHOLD_SEARCH}ms)"
echo "  simulate_transaction: $(cat /tmp/perf_simulate_transaction.txt 2>/dev/null || echo 'N/A')ms (target: <${THRESHOLD_SIMULATE}ms)"
echo "  get_defi_positions:   $(cat /tmp/perf_get_defi_positions.txt 2>/dev/null || echo 'N/A')ms (target: <${THRESHOLD_DEFI}ms)"
if [[ -n "${TEST_TX_HASH}" ]]; then
echo "  decode_transaction:   $(cat /tmp/perf_decode_transaction.txt 2>/dev/null || echo 'N/A')ms (target: <${THRESHOLD_DECODE}ms)"
fi
echo "  get_account_summary:  $(cat /tmp/perf_get_account_summary.txt 2>/dev/null || echo 'N/A')ms (target: <${THRESHOLD_ACCOUNT}ms)"
echo "  construct_swap_tx:    $(cat /tmp/perf_construct_swap_tx.txt 2>/dev/null || echo 'N/A')ms (target: <${THRESHOLD_SWAP}ms)"
echo ""

# Cleanup temp files
rm -f /tmp/perf_*.txt

echo "[perf] OK"

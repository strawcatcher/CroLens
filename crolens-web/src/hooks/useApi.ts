import { useMutation, useQuery } from "@tanstack/react-query";
import {
  constructSwapTx,
  decodeTransaction,
  getAccountSummary,
  getDefiPositions,
  searchContract,
  simulateTransaction,
} from "@/lib/api";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import { useAppStore } from "@/stores/app";

export function useAccountSummary(
  address: string | undefined,
  simpleMode = false,
) {
  const apiKey = useAppStore((s) => s.apiKey);
  return useQuery({
    queryKey: ["get_account_summary", address, simpleMode],
    queryFn: () =>
      getAccountSummary({ address: address!, simpleMode }, { apiKey }),
    enabled: typeof address === "string" && address.trim().length > 0,
    staleTime: 30_000,
    gcTime: 300_000,
  });
}

export function useDefiPositions(
  address: string | undefined,
  simpleMode = false,
) {
  const apiKey = useAppStore((s) => s.apiKey);
  return useQuery({
    queryKey: ["get_defi_positions", address, simpleMode],
    queryFn: () =>
      getDefiPositions({ address: address!, simpleMode }, { apiKey }),
    enabled: typeof address === "string" && address.trim().length > 0,
    staleTime: 30_000,
    gcTime: 300_000,
  });
}

export function useDecodeTransaction(
  txHash: string | undefined,
  simpleMode = false,
) {
  const apiKey = useAppStore((s) => s.apiKey);
  return useQuery({
    queryKey: ["decode_transaction", txHash, simpleMode],
    queryFn: () =>
      decodeTransaction({ txHash: txHash!, simpleMode }, { apiKey }),
    enabled: typeof txHash === "string" && txHash.trim().length > 0,
    staleTime: 60_000,
    gcTime: 300_000,
  });
}

export function useSearchContract(query: string, limit = 20) {
  const apiKey = useAppStore((s) => s.apiKey);
  const debounced = useDebouncedValue(query, 300);
  return useQuery({
    queryKey: ["search_contract", debounced, limit],
    queryFn: () => searchContract({ query: debounced, limit }, { apiKey }),
    enabled: debounced.trim().length > 0,
    staleTime: 60_000,
    gcTime: 300_000,
  });
}

export function useSimulateTransaction() {
  const apiKey = useAppStore((s) => s.apiKey);
  return useMutation({
    mutationFn: (args: {
      from: string;
      to: string;
      data: string;
      value: string;
      simpleMode?: boolean;
    }) => simulateTransaction(args, { apiKey }),
  });
}

export function useConstructSwap() {
  const apiKey = useAppStore((s) => s.apiKey);
  return useMutation({
    mutationFn: (args: {
      from: string;
      tokenIn: string;
      tokenOut: string;
      amountIn: string;
      slippageBps: number;
    }) => constructSwapTx(args, { apiKey }),
  });
}

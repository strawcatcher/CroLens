import * as React from "react";
import { useQuery } from "@tanstack/react-query";
import { motion, useReducedMotion } from "framer-motion";
import { P5Title, P5Card, StatCard } from "@/components/p5";
import { fetchHealth, fetchStats } from "@/lib/api";
import { useAppStore } from "@/stores/app";

function formatTime(ts: number) {
  return new Date(ts).toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function formatPercent(value: number) {
  return `${value.toFixed(1)}%`;
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

function StatusBadge({ status }: { status: string }) {
  const isOk = status === "ok";
  const isDegraded = status === "degraded";
  const isUnhealthy = status === "unhealthy";

  const label = isOk
    ? "ONLINE"
    : isDegraded
      ? "DEGRADED"
      : isUnhealthy
        ? "OFFLINE"
        : "UNKNOWN";

  const dotColor = isOk
    ? "bg-[#00FF41]"
    : isDegraded
      ? "bg-[#FFD700]"
      : isUnhealthy
        ? "bg-[#FF4444]"
        : "bg-[#FFD700]";

  const textColor = isOk
    ? "text-[#00FF41]"
    : isDegraded
      ? "text-[#FFD700]"
      : isUnhealthy
        ? "text-[#FF4444]"
        : "text-[#FFD700]";

  return (
    <div
      className="inline-flex items-center gap-2 font-mono text-xs tracking-widest"
      aria-label={`Backend status: ${label}`}
    >
      <span
        className={`w-2 h-2 rounded-full ${dotColor}`}
        aria-hidden="true"
      />
      <span className={textColor}>{label}</span>
    </div>
  );
}

function DashboardStatCard({
  label,
  value,
  format,
  trendValue,
  trend,
  isPositive = true,
}: {
  label: string;
  value: number;
  format?: (v: number) => string;
  trendValue?: string;
  trend?: string;
  isPositive?: boolean;
}) {
  const formattedValue = format ? format(value) : String(Math.round(value));
  return (
    <StatCard
      label={label}
      value={formattedValue}
      trendValue={trendValue}
      trend={trend}
      isPositive={isPositive}
    />
  );
}

export function DashboardPage() {
  const latency = useAppStore((s) => s.latency);

  const totalRequests = latency.length;
  const avgLatency = React.useMemo(() => {
    if (latency.length === 0) return 0;
    const sum = latency.reduce((acc, v) => acc + v.latencyMs, 0);
    return Math.round(sum / latency.length);
  }, [latency]);

  const errorRate = React.useMemo(() => {
    if (latency.length === 0) return 0;
    const errors = latency.filter((s) => s.status === "error").length;
    return clamp((errors / latency.length) * 100, 0, 100);
  }, [latency]);

  const chartData = React.useMemo(
    () =>
      latency.slice(-60).map((s) => ({
        time: formatTime(s.ts),
        latencyMs: s.latencyMs,
        tool: s.tool,
        status: s.status,
      })),
    [latency],
  );

  // 预先计算最大延迟，避免在 map 中重复计算
  const chartDataForDisplay = React.useMemo(() => {
    const last20 = chartData.slice(-20);
    const maxLatency = Math.max(...last20.map((c) => c.latencyMs), 100);
    return last20.map((d) => ({
      ...d,
      heightPercent: (d.latencyMs / maxLatency) * 100,
    }));
  }, [chartData]);

  // 稳定的 SESSION ID，只在组件首次挂载时生成
  const sessionId = React.useMemo(
    () => Math.random().toString(36).substring(2, 10).toUpperCase(),
    [],
  );

  const health = useQuery({
    queryKey: ["health"],
    queryFn: () => fetchHealth(),
    refetchInterval: 30_000,
    retry: false,
  });

  const stats = useQuery({
    queryKey: ["stats"],
    queryFn: () => fetchStats(),
    refetchInterval: 60_000,
    retry: false,
  });

  const status = health.data?.status ?? "unknown";
  const protocolsSupported = stats.data?.protocols_supported;

  const reducedMotion = useReducedMotion();

  const recentLatency = React.useMemo(
    () => latency.slice(-20).reverse(),
    [latency],
  );

  return (
    <div className="space-y-8">
      <P5Title subTitle="Real-time metrics & node status">
        DASHBOARD
      </P5Title>

      {/* 状态栏 */}
      <div className="flex flex-wrap items-center justify-between gap-4 mb-2">
        <StatusBadge status={status} />
        <div className="font-mono text-xs text-[#555]">
          SESSION ID: #{sessionId}
        </div>
      </div>

      {/* 指标网格 */}
      <motion.div
        className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-4"
        initial={reducedMotion ? undefined : "hidden"}
        animate={reducedMotion ? undefined : "show"}
        variants={{
          hidden: { opacity: 0 },
          show: {
            opacity: 1,
            transition: { staggerChildren: 0.05 },
          },
        }}
      >
        <motion.div
          variants={{
            hidden: { opacity: 0, y: 20 },
            show: { opacity: 1, y: 0, transition: { duration: 0.3, ease: [0, 0, 0.2, 1] } },
          }}
        >
          <DashboardStatCard
            label="TOTAL CALLS"
            value={totalRequests}
            trendValue={totalRequests > 0 ? `+${Math.min(totalRequests, 100)}%` : undefined}
            trend="this session"
            isPositive={true}
          />
        </motion.div>
        <motion.div
          variants={{
            hidden: { opacity: 0, y: 20 },
            show: { opacity: 1, y: 0, transition: { duration: 0.3, ease: [0, 0, 0.2, 1] } },
          }}
        >
          <DashboardStatCard
            label="AVG LATENCY"
            value={avgLatency}
            format={(v) => `${Math.round(v)}ms`}
            trendValue={avgLatency > 0 ? `${avgLatency < 100 ? '-' : '+'}${Math.abs(avgLatency - 100)}ms` : undefined}
            trend="vs target"
            isPositive={avgLatency < 150}
          />
        </motion.div>
        <motion.div
          variants={{
            hidden: { opacity: 0, y: 20 },
            show: { opacity: 1, y: 0, transition: { duration: 0.3, ease: [0, 0, 0.2, 1] } },
          }}
        >
          <DashboardStatCard
            label="ERROR RATE"
            value={errorRate}
            format={formatPercent}
            trendValue={errorRate <= 1 ? "STABLE" : "HIGH"}
            trend="threshold 1%"
            isPositive={errorRate <= 1}
          />
        </motion.div>
        <motion.div
          variants={{
            hidden: { opacity: 0, y: 20 },
            show: { opacity: 1, y: 0, transition: { duration: 0.3, ease: [0, 0, 0.2, 1] } },
          }}
        >
          <DashboardStatCard
            label="PROTOCOLS"
            value={typeof protocolsSupported === "number" ? protocolsSupported : 0}
            format={(v) =>
              typeof protocolsSupported === "number" ? String(Math.round(v)) : "—"
            }
            trendValue="ACTIVE"
            trend="supported"
            isPositive={true}
          />
        </motion.div>
      </motion.div>

      {/* 图表区域 */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2">
          <P5Card title="TRAFFIC ANALYSIS">
            {chartData.length === 0 ? (
              <div className="h-64 flex items-center justify-center text-[#555] font-mono">
                No data yet. Execute some tools in Playground.
              </div>
            ) : (
              <div className="h-64 relative">
                {/* 网格线背景 */}
                <div
                  className="absolute inset-0 border-b border-l border-[#333] z-0 opacity-50 p5-grid-bg"
                />

                {/* 模拟柱状图 */}
                <div className="h-full flex items-end justify-between gap-1 px-2 pb-2 relative z-10">
                  {chartDataForDisplay.map((d) => (
                    <div
                      key={`${d.time}-${d.latencyMs}`}
                      className="w-full bg-[#D90018] relative group hover:bg-white"
                      style={{ height: `${Math.max(d.heightPercent, 5)}%` }}
                    >
                      <div className="absolute -top-6 left-1/2 -translate-x-1/2 bg-white text-black text-[10px] px-1 font-mono opacity-0 group-hover:opacity-100 whitespace-nowrap z-20 pointer-events-none">
                        {d.latencyMs}ms
                      </div>
                    </div>
                  ))}
                </div>
                <div className="flex justify-between text-[#555] font-mono text-xs mt-2 px-2">
                  <span>{chartData[0]?.time ?? '--'}</span>
                  <span>{chartData[Math.floor(chartData.length / 2)]?.time ?? '--'}</span>
                  <span>{chartData[chartData.length - 1]?.time ?? '--'}</span>
                </div>
              </div>
            )}
          </P5Card>
        </div>

        <div className="lg:col-span-1">
          <P5Card title="ACTIVE NODES">
            <div className="space-y-0 text-sm font-mono">
              {[
                { name: "CRONOS_MAIN_A", status: status === "ok" ? "OK" : "WARN", ping: `${avgLatency || 24}ms` },
                { name: "CRONOS_MAIN_B", status: "OK", ping: "32ms" },
                { name: "CRONOS_TEST_1", status: errorRate > 1 ? "WARN" : "OK", ping: "145ms" },
                { name: "ARCHIVE_NODE", status: "OK", ping: "89ms" },
              ].map((node, i) => (
                <div
                  key={i}
                  className="flex justify-between items-center py-3 border-b border-[#333] last:border-0 hover:bg-white/5 px-2"
                >
                  <span className="text-[#A3A3A3]">{node.name}</span>
                  <div className="flex items-center gap-3">
                    <span className={node.status === 'OK' ? 'text-[#00FF41]' : 'text-[#FFD700]'}>
                      {node.ping}
                    </span>
                    <div className={`w-2 h-2 rounded-sm ${node.status === 'OK' ? 'bg-[#00FF41]' : 'bg-[#FFD700]'}`} />
                  </div>
                </div>
              ))}
            </div>
          </P5Card>
        </div>
      </div>

      {/* 实时日志 */}
      <P5Card title="LIVE LOG STREAM">
        <div className="max-h-72 overflow-auto">
          {recentLatency.length === 0 ? (
            <div className="text-sm text-[#555] font-mono">No logs yet.</div>
          ) : (
            <ul className="space-y-2 text-xs">
              {recentLatency.map((l) => {
                const width = clamp((l.latencyMs / 300) * 100, 0, 100);
                const statusColor =
                  l.status === "success" ? "text-[#00FF41]" : "text-[#FF4444]";
                return (
                  <li
                    key={`${l.ts}-${l.tool}-${l.latencyMs}`}
                    className="grid grid-cols-[72px_1fr_60px_120px] items-center gap-3 font-mono text-[#A3A3A3] py-2 border-b border-[#333] last:border-0 hover:bg-white/5"
                  >
                    <div className="text-[#555]">{formatTime(l.ts)}</div>
                    <div className="truncate text-white">{l.tool}</div>
                    <div className={`text-right ${statusColor}`}>
                      {l.status.toUpperCase()}
                    </div>
                    <div className="flex items-center justify-end gap-2">
                      <span className="w-10 text-right tabular-nums text-[#A3A3A3]">
                        {l.latencyMs}ms
                      </span>
                      <div className="h-1 w-[60px] overflow-hidden bg-[#333]">
                        <div
                          className="h-full bg-[#D90018]"
                          style={{ width: `${width}%` }}
                        />
                      </div>
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </P5Card>
    </div>
  );
}

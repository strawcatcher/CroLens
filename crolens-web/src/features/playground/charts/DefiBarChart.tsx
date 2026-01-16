import { BarChart, Bar, XAxis, YAxis, ResponsiveContainer, Tooltip, Cell } from 'recharts';

// P5 color palette for protocols
const PROTOCOL_COLORS: Record<string, string> = {
  VVS: '#D90018',
  Tectonic: '#00FF41',
  Ferro: '#FFD700',
  MM: '#0A84FF',
  default: '#A3A3A3',
};

type DefiData = {
  protocol: string;
  valueUsd: number;
  type?: string;
};

type DefiBarChartProps = {
  data: DefiData[];
  className?: string;
};

function CustomTooltip({ active, payload }: { active?: boolean; payload?: Array<{ payload: DefiData }> }) {
  if (!active || !payload || payload.length === 0) return null;

  const item = payload[0].payload;
  return (
    <div className="bg-black border border-[#D90018] p-3 transform -skew-x-6">
      <div className="skew-x-6">
        <div className="font-bebas text-white tracking-wider">{item.protocol}</div>
        {item.type && (
          <div className="font-mono text-xs text-[#555] uppercase">{item.type}</div>
        )}
        <div className="font-mono text-sm text-[#D90018]">
          ${item.valueUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
        </div>
      </div>
    </div>
  );
}

export function DefiBarChart({ data, className = '' }: DefiBarChartProps) {
  if (data.length === 0) {
    return (
      <div className={`flex items-center justify-center h-full text-[#555] font-mono ${className}`}>
        No DeFi positions
      </div>
    );
  }

  const total = data.reduce((sum, item) => sum + item.valueUsd, 0);

  return (
    <div className={`relative ${className}`}>
      {/* P5 decorative elements */}
      <div className="absolute top-0 left-0 w-full h-[2px] bg-gradient-to-r from-[#D90018] via-transparent to-transparent" />

      {/* Total value header */}
      <div className="mb-4 flex items-center justify-between">
        <div className="font-bebas text-[#A3A3A3] tracking-wider">PROTOCOL DISTRIBUTION</div>
        <div className="font-bebas text-xl text-white">
          ${total.toLocaleString(undefined, { maximumFractionDigits: 0 })}
        </div>
      </div>

      <ResponsiveContainer width="100%" height={data.length * 48 + 20}>
        <BarChart
          data={data}
          layout="vertical"
          margin={{ top: 0, right: 0, left: 0, bottom: 0 }}
        >
          <XAxis
            type="number"
            hide
            domain={[0, 'dataMax']}
          />
          <YAxis
            type="category"
            dataKey="protocol"
            axisLine={false}
            tickLine={false}
            tick={{ fill: '#A3A3A3', fontSize: 12, fontFamily: 'Bebas Neue' }}
            width={80}
          />
          <Tooltip content={<CustomTooltip />} cursor={false} />
          <Bar
            dataKey="valueUsd"
            radius={[0, 2, 2, 0]}
            barSize={24}
          >
            {data.map((entry, index) => (
              <Cell
                key={`cell-${index}`}
                fill={PROTOCOL_COLORS[entry.protocol] || PROTOCOL_COLORS.default}
                className="transition-opacity hover:opacity-80"
              />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>

      {/* Value labels */}
      <div className="mt-2 space-y-1">
        {data.map((item, index) => {
          const percent = total > 0 ? (item.valueUsd / total) * 100 : 0;
          const color = PROTOCOL_COLORS[item.protocol] || PROTOCOL_COLORS.default;
          return (
            <div key={index} className="flex items-center justify-between text-xs">
              <div className="flex items-center gap-2">
                <div
                  className="w-3 h-3 transform rotate-45 border"
                  style={{ borderColor: color, backgroundColor: `${color}33` }}
                />
                <span className="font-mono text-[#A3A3A3]">{item.protocol}</span>
                {item.type && (
                  <span className="font-mono text-[#555]">({item.type})</span>
                )}
              </div>
              <div className="flex items-center gap-2">
                <span className="font-mono text-white tabular-nums">
                  ${item.valueUsd.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                </span>
                <span className="font-mono text-[#555] w-12 text-right">
                  {percent.toFixed(1)}%
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

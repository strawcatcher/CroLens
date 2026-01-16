import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from 'recharts';

// P5 color palette
const P5_COLORS = [
  '#D90018', // Primary red
  '#FF4444', // Light red
  '#00FF41', // Success green
  '#FFD700', // Warning yellow
  '#0A84FF', // Info blue
  '#A3A3A3', // Gray
  '#FF6B6B', // Coral
  '#9B59B6', // Purple
];

type AssetData = {
  symbol: string;
  valueUsd: number;
};

type AssetPieChartProps = {
  data: AssetData[];
  className?: string;
};

function CustomTooltip({ active, payload }: { active?: boolean; payload?: Array<{ payload: AssetData & { percent: number } }> }) {
  if (!active || !payload || payload.length === 0) return null;

  const item = payload[0].payload;
  return (
    <div className="bg-black border border-[#D90018] p-3 transform -skew-x-6">
      <div className="skew-x-6">
        <div className="font-bebas text-white tracking-wider">{item.symbol}</div>
        <div className="font-mono text-sm text-[#A3A3A3]">
          ${item.valueUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
        </div>
        <div className="font-mono text-xs text-[#D90018]">
          {(item.percent * 100).toFixed(1)}%
        </div>
      </div>
    </div>
  );
}

export function AssetPieChart({ data, className = '' }: AssetPieChartProps) {
  if (data.length === 0) {
    return (
      <div className={`flex items-center justify-center h-full text-[#555] font-mono ${className}`}>
        No assets to display
      </div>
    );
  }

  const total = data.reduce((sum, item) => sum + item.valueUsd, 0);
  const chartData = data.map((item) => ({
    ...item,
    percent: total > 0 ? item.valueUsd / total : 0,
  }));

  return (
    <div className={`relative ${className}`}>
      {/* P5 decorative corner */}
      <div className="absolute top-0 right-0 w-6 h-6 border-t-2 border-r-2 border-[#D90018] opacity-50" />
      <div className="absolute bottom-0 left-0 w-6 h-6 border-b-2 border-l-2 border-[#D90018] opacity-50" />

      <ResponsiveContainer width="100%" height="100%">
        <PieChart>
          <Pie
            data={chartData}
            cx="50%"
            cy="50%"
            innerRadius="40%"
            outerRadius="70%"
            paddingAngle={2}
            dataKey="valueUsd"
            stroke="#000"
            strokeWidth={2}
          >
            {chartData.map((_, index) => (
              <Cell
                key={`cell-${index}`}
                fill={P5_COLORS[index % P5_COLORS.length]}
                className="transition-opacity hover:opacity-80"
              />
            ))}
          </Pie>
          <Tooltip content={<CustomTooltip />} />
        </PieChart>
      </ResponsiveContainer>

      {/* Center label */}
      <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
        <div className="text-center">
          <div className="font-bebas text-2xl text-white tracking-wider">
            ${total.toLocaleString(undefined, { maximumFractionDigits: 0 })}
          </div>
          <div className="font-mono text-xs text-[#A3A3A3]">TOTAL</div>
        </div>
      </div>

      {/* Legend */}
      <div className="absolute bottom-0 left-0 right-0 flex flex-wrap justify-center gap-2 pb-2">
        {chartData.slice(0, 4).map((item, index) => (
          <div key={item.symbol} className="flex items-center gap-1">
            <div
              className="w-2 h-2 transform rotate-45"
              style={{ backgroundColor: P5_COLORS[index % P5_COLORS.length] }}
            />
            <span className="font-mono text-xs text-[#A3A3A3]">{item.symbol}</span>
          </div>
        ))}
        {chartData.length > 4 && (
          <span className="font-mono text-xs text-[#555]">+{chartData.length - 4} more</span>
        )}
      </div>
    </div>
  );
}

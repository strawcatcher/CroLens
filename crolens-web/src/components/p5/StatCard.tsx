type StatCardProps = {
  label: string;
  value: string;
  trend?: string;
  trendValue?: string;
  isPositive?: boolean;
};

export function StatCard({ label, value, trend, trendValue, isPositive = true }: StatCardProps) {
  return (
    <div className="bg-[#1A1A1A] p5-halftone-strong p-4 border-l-2 border-[#D90018] relative overflow-hidden group transition-all duration-200 hover:border-l-4 hover:bg-[#1E1E1E]">
      {/* 装饰菱形 - 增强 */}
      <div className="absolute top-0 right-0 p-1 opacity-20 group-hover:opacity-40 transition-opacity">
        <div className="w-8 h-8 border border-white transform rotate-45 translate-x-4 -translate-y-4" />
        <div className="w-4 h-4 border border-[#D90018] transform rotate-45 translate-x-6 translate-y-0" />
      </div>

      {/* 底部红色渐变装饰 */}
      <div className="absolute bottom-0 left-0 right-0 h-[2px] bg-gradient-to-r from-[#D90018] via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />

      <div className="font-bebas text-[#A3A3A3] tracking-wider text-lg mb-1 group-hover:text-[#B3B3B3] transition-colors">{label}</div>
      <div className="font-bebas text-3xl text-white mb-2 tabular-nums tracking-wide group-hover:scale-105 group-hover:text-shadow transition-all origin-left">
        {value}
      </div>
      {trendValue && (
        <div className={`font-mono text-xs flex items-center gap-1 ${isPositive ? 'text-[#00FF41]' : 'text-[#FFD700]'}`}>
          <span className="inline-block group-hover:scale-125 transition-transform">
            {isPositive ? '▲' : '▼'}
          </span>
          {trendValue}
          {trend && <span className="opacity-50 ml-1">{trend}</span>}
        </div>
      )}
    </div>
  );
}

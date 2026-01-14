type StatCardProps = {
  label: string;
  value: string;
  trend?: string;
  trendValue?: string;
  isPositive?: boolean;
};

export function StatCard({ label, value, trend, trendValue, isPositive = true }: StatCardProps) {
  return (
    <div className="bg-[#1A1A1A] p-4 border-l-2 border-[#D90018] relative overflow-hidden group">
      <div className="absolute top-0 right-0 p-1 opacity-20">
        <div className="w-8 h-8 border border-white transform rotate-45 translate-x-4 -translate-y-4" />
      </div>
      <div className="font-bebas text-[#A3A3A3] tracking-wider text-lg mb-1">{label}</div>
      <div className="font-bebas text-3xl text-white mb-2 tabular-nums tracking-wide group-hover:scale-105 transition-transform origin-left">
        {value}
      </div>
      {trendValue && (
        <div className={`font-mono text-xs flex items-center gap-1 ${isPositive ? 'text-[#00FF41]' : 'text-[#FFD700]'}`}>
          {isPositive ? '▲' : '▼'} {trendValue}
          {trend && <span className="opacity-50 ml-1">{trend}</span>}
        </div>
      )}
    </div>
  );
}

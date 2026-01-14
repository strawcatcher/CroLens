type P5CardProps = {
  title: string;
  children: React.ReactNode;
  className?: string;
  headerAction?: React.ReactNode;
};

export function P5Card({ title, children, className = '', headerAction }: P5CardProps) {
  return (
    <div className={`relative group ${className}`}>
      {/* 装饰性背景层 (hover offset shadow) */}
      <div
        className="absolute inset-0 bg-[#D90018] transform translate-x-1 translate-y-1 -z-10 opacity-0 group-hover:opacity-100 transition-opacity duration-300"
        style={{ clipPath: 'polygon(0 0, calc(100% - 24px) 0, 100% 24px, 100% 100%, 0 100%)' }}
      />

      {/* 主卡片 */}
      <div
        className="relative bg-[#121212] border-l-2 border-[#333] h-full flex flex-col"
        style={{ clipPath: 'polygon(0 0, calc(100% - 24px) 0, 100% 24px, 100% 100%, 0 100%)' }}
      >
        {/* 顶部红线 */}
        <div className="h-[3px] w-full bg-[#D90018] mb-4" />

        {/* 标题区 */}
        <div className="px-6 flex justify-between items-start mb-4">
          <div className="bg-[#1A1A1A] px-4 py-1 transform -skew-x-12 border-l-4 border-[#D90018]">
            <h3 className="transform skew-x-12 font-bebas tracking-widest text-xl text-white">
              {title}
            </h3>
          </div>
          {headerAction && <div className="transform translate-y-1">{headerAction}</div>}
        </div>

        {/* 内容区 */}
        <div className="px-6 pb-6 flex-1 relative z-10">
          <div className="bg-[#1A1A1A]/50 rounded-sm p-4 h-full border border-white/5 backdrop-blur-sm">
            {children}
          </div>
        </div>

        {/* 底部装饰 */}
        <div className="absolute bottom-0 right-0 w-8 h-8 bg-gradient-to-tl from-[#D90018]/20 to-transparent pointer-events-none" />
      </div>
    </div>
  );
}

type P5CardProps = {
  title: string;
  children: React.ReactNode;
  className?: string;
  headerAction?: React.ReactNode;
  /** 使用锯齿边缘样式 */
  jagged?: boolean;
};

export function P5Card({ title, children, className = '', headerAction, jagged = false }: P5CardProps) {
  const clipClass = jagged ? 'p5-clip-card-jagged' : 'p5-clip-card';

  return (
    <div className={`relative group p5-card-hover ${className}`}>
      {/* 装饰性背景层 (hover offset shadow) */}
      <div
        className={`absolute inset-0 bg-[#D90018] translate-x-1 translate-y-1 -z-10 opacity-0 group-hover:opacity-60 transition-all duration-300 group-hover:translate-x-2 group-hover:translate-y-2 ${clipClass}`}
      />

      {/* 主卡片 */}
      <div
        className={`relative bg-[#121212] border-l-2 border-[#333] h-full flex flex-col ${clipClass}`}
      >
        {/* 顶部红线 */}
        <div className="h-[3px] w-full bg-[#D90018] mb-4" />

        {/* 标题区 */}
        <div className="px-6 flex justify-between items-start mb-4">
          <div className="bg-[#1A1A1A] px-4 py-1 transform -skew-x-12 border-l-4 border-[#D90018] shadow-[2px_2px_0_rgba(217,0,24,0.3)]">
            <h3 className="transform skew-x-12 font-bebas tracking-widest text-xl text-white">
              {title}
            </h3>
          </div>
          {headerAction && <div className="transform translate-y-1">{headerAction}</div>}
        </div>

        {/* 内容区 - 启用 halftone 背景 */}
        <div className="px-6 pb-6 flex-1 relative z-10 overflow-hidden min-h-0">
          <div className="bg-[#1A1A1A] p5-halftone-bg rounded-sm p-4 h-full border border-white/5 overflow-auto">
            {children}
          </div>
        </div>

        {/* 底部装饰 - 增强 */}
        <div className="absolute bottom-0 right-0 w-12 h-12 bg-gradient-to-tl from-[#D90018]/30 to-transparent pointer-events-none" />
        <div className="absolute top-0 left-0 w-8 h-8 bg-gradient-to-br from-white/5 to-transparent pointer-events-none" />
      </div>
    </div>
  );
}

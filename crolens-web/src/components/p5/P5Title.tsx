import { Zap } from 'lucide-react';

type P5TitleProps = {
  children: React.ReactNode;
  subTitle?: string;
  className?: string;
};

export function P5Title({ children, subTitle, className = '' }: P5TitleProps) {
  return (
    <div className={`mb-8 ${className}`}>
      {/* 主标题容器 - 增强阴影效果 */}
      <div className="relative inline-block">
        {/* 多层偏移阴影 - 更酷的立体感 */}
        <div className="absolute inset-0 transform -skew-x-6 bg-black translate-x-2 translate-y-2 opacity-40" />
        <div className="absolute inset-0 transform -skew-x-6 bg-[#D90018] translate-x-1 translate-y-1 opacity-60" />

        {/* 主体 */}
        <div className="relative transform -skew-x-6 bg-[#D90018] px-6 py-2 border-b-2 border-black/30">
          {/* 条纹装饰叠加 */}
          <div className="absolute inset-0 p5-bar-stripe opacity-20 pointer-events-none" />

          <h1 className="relative transform skew-x-6 text-3xl md:text-4xl tracking-[0.2em] font-bebas text-white uppercase p5-text-stroke">
            {children}
          </h1>
        </div>
      </div>

      {subTitle && (
        <div className="mt-3 ml-1 text-[#A3A3A3] font-inter text-sm md:text-base tracking-wide flex items-center gap-2">
          <div className="w-4 h-4 bg-[#D90018]/20 flex items-center justify-center transform rotate-45">
            <Zap size={10} className="text-[#D90018] transform -rotate-45" />
          </div>
          {subTitle}
        </div>
      )}
    </div>
  );
}

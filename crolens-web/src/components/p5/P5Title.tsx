import { Zap } from 'lucide-react';

type P5TitleProps = {
  children: React.ReactNode;
  subTitle?: string;
  className?: string;
};

export function P5Title({ children, subTitle, className = '' }: P5TitleProps) {
  return (
    <div className={`mb-8 ${className}`}>
      <div className="relative inline-block transform -skew-x-6 bg-[#D90018] px-6 py-2 shadow-[4px_4px_0px_0px_rgba(255,255,255,0.2)]">
        <h1 className="transform skew-x-6 text-3xl md:text-4xl tracking-[0.2em] font-bebas text-white uppercase">
          {children}
        </h1>
      </div>
      {subTitle && (
        <div className="mt-2 ml-1 text-[#A3A3A3] font-inter text-sm md:text-base tracking-wide flex items-center gap-2">
          <Zap size={14} className="text-[#D90018]" />
          {subTitle}
        </div>
      )}
    </div>
  );
}

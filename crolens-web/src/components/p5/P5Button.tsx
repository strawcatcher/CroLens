import { cn } from '@/lib/utils';
import { Loader2 } from 'lucide-react';

type P5ButtonProps = {
  children: React.ReactNode;
  variant?: 'primary' | 'secondary';
  onClick?: () => void;
  icon?: React.ComponentType<{ size?: number | string; strokeWidth?: number | string; className?: string }>;
  className?: string;
  disabled?: boolean;
  loading?: boolean;
  type?: 'button' | 'submit';
};

export function P5Button({
  children,
  variant = 'primary',
  onClick,
  icon: Icon,
  className = '',
  disabled = false,
  loading = false,
  type = 'button',
}: P5ButtonProps) {
  const isDisabled = disabled || loading;

  if (variant === 'primary') {
    return (
      <button
        type={type}
        onClick={onClick}
        disabled={isDisabled}
        className={cn(
          "group relative inline-flex items-center justify-center px-8 py-3 font-bebas text-xl tracking-widest uppercase transition-transform active:scale-95 outline-none focus:ring-2 focus:ring-[#D90018] focus:ring-offset-2 focus:ring-offset-black",
          isDisabled && "opacity-50 cursor-not-allowed",
          className
        )}
      >
        {/* 阴影层 */}
        <span
          className="absolute inset-0 bg-[#D90018] translate-x-1 translate-y-1 opacity-60 group-hover:translate-x-2 group-hover:translate-y-2 transition-transform duration-200 p5-clip-button"
        />
        {/* 主体层 */}
        <span
          className="absolute inset-0 bg-[#D90018] group-hover:bg-[#ff1a35] transition-colors p5-clip-button"
        />
        {/* 文字内容 */}
        <span className="relative z-10 flex items-center gap-2 text-black">
          {loading ? (
            <Loader2 size={20} className="animate-spin" />
          ) : Icon ? (
            <Icon size={20} strokeWidth={2.5} />
          ) : null}
          {children}
        </span>
      </button>
    );
  }

  // Ghost/Secondary Button
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={isDisabled}
      className={cn(
        "relative px-4 py-2 font-mono text-sm uppercase text-[#A3A3A3] hover:text-white border border-transparent hover:border-white/20 hover:bg-white/5 transition-all flex items-center gap-2 group outline-none focus:bg-[#D90018]/20",
        isDisabled && "opacity-50 cursor-not-allowed",
        className
      )}
    >
      <span className="absolute left-0 top-0 h-2 w-[1px] bg-current opacity-0 group-hover:opacity-100 transition-opacity" />
      <span className="absolute right-0 bottom-0 h-2 w-[1px] bg-current opacity-0 group-hover:opacity-100 transition-opacity" />
      {loading ? (
        <Loader2 size={16} className="animate-spin" />
      ) : Icon ? (
        <Icon size={16} />
      ) : null}
      {children}
    </button>
  );
}

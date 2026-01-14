import { ArrowRight } from 'lucide-react';
import { cn } from '@/lib/utils';

export type ToolSelectorOption = {
  value: string;
  label: string;
};

export type ToolSelectorProps = {
  value: string;
  options: ToolSelectorOption[];
  onValueChange: (value: string) => void;
  className?: string;
};

export function ToolSelector({ value, options, onValueChange, className }: ToolSelectorProps) {
  return (
    <div className={cn("flex flex-col gap-1 w-full h-full overflow-y-auto pr-2", className)}>
      {options.map((tool) => {
        const isActive = value === tool.value;
        return (
          <button
            key={tool.value}
            onClick={() => onValueChange(tool.value)}
            className={cn(
              "relative group w-full text-left px-4 py-3 transition-all outline-none focus:ring-1 focus:ring-white/50",
              isActive ? "pl-6" : "hover:pl-5"
            )}
          >
            {isActive && (
              <div className="absolute inset-0 bg-[#D90018] transform -skew-x-12 p5-animate-slide-in origin-left z-0" />
            )}
            <div className="relative z-10 flex items-baseline justify-between">
              <span className={cn(
                "font-mono text-sm md:text-base truncate transition-colors",
                isActive ? "text-black font-bold" : "text-[#A3A3A3] group-hover:text-white"
              )}>
                {tool.label}
              </span>
              {isActive && <ArrowRight size={14} className="text-black ml-2" />}
            </div>
          </button>
        );
      })}
    </div>
  );
}

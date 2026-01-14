import { cn } from "@/lib/utils";

export type ProgressProps = {
  value: number;
  max?: number;
  className?: string;
  "aria-label"?: string;
};

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

function getFillClass(ratio: number) {
  if (ratio > 0.5) return "bg-success";
  if (ratio > 0.2) return "bg-warning";
  return "bg-destructive";
}

export function Progress({
  value,
  max = 100,
  className,
  "aria-label": ariaLabel = "Progress",
}: ProgressProps) {
  const safeMax = Number.isFinite(max) && max > 0 ? max : 100;
  const safeValue = Number.isFinite(value) ? value : 0;
  const ratio = clamp(safeValue / safeMax, 0, 1);
  const percent = Math.round(ratio * 100);

  return (
    <div
      className={cn(
        "h-2 w-full overflow-hidden rounded-sm bg-elevated",
        className,
      )}
      role="progressbar"
      aria-label={ariaLabel}
      aria-valuemin={0}
      aria-valuemax={safeMax}
      aria-valuenow={safeValue}
    >
      <div
        className={cn(
          "h-full transition-[width,background-color] duration-[var(--duration-slower)] ease-[var(--ease-out)]",
          getFillClass(ratio),
        )}
        style={{ width: `${percent}%` }}
      />
    </div>
  );
}

import * as React from "react";
import { cn } from "@/lib/utils";

const Input = React.forwardRef<
  HTMLInputElement,
  React.InputHTMLAttributes<HTMLInputElement>
>(({ className, type, ...props }, ref) => (
  <input
    type={type}
    className={cn(
      "flex h-12 w-full rounded-sm border-2 border-border bg-elevated px-4 text-sm text-foreground placeholder:text-muted transition-[border-color,box-shadow,transform] duration-[var(--duration-normal)] ease-[var(--ease-out)] focus-visible:outline-none focus-visible:border-[var(--border-focus)] focus-visible:shadow-[0_0_0_3px_var(--accent-red-glow)] disabled:cursor-not-allowed disabled:opacity-40 aria-[invalid=true]:border-destructive aria-[invalid=true]:animate-[crolens-shake_300ms_var(--ease-out)]",
      className,
    )}
    ref={ref}
    {...props}
  />
));
Input.displayName = "Input";

export { Input };

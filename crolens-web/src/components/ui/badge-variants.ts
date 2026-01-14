import { cva } from "class-variance-authority";

export const badgeVariants = cva(
  "inline-flex items-center gap-1 whitespace-nowrap rounded-sm border border-border px-2 py-1 text-xs font-mono tracking-[0.12em] uppercase leading-none text-foreground",
  {
    variants: {
      variant: {
        default: "bg-card",
        secondary: "bg-elevated text-muted-foreground",
        primary: "border-primary/40 bg-primary text-white",
        success: "border-success/40 bg-success/10 text-success",
        warning: "border-warning/40 bg-warning/10 text-warning",
        destructive: "border-destructive/40 bg-destructive/10 text-destructive",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);

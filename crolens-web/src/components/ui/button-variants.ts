import { cva } from "class-variance-authority";

export const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm transition-[transform,background-color,border-color,box-shadow,color,opacity] duration-[var(--duration-normal)] ease-[var(--ease-out)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 focus-visible:ring-offset-2 focus-visible:ring-offset-black disabled:pointer-events-none disabled:opacity-40 disabled:shadow-none disabled:hover:translate-x-0 disabled:hover:translate-y-0 will-change-transform",
  {
    variants: {
      variant: {
        default:
          "p5-clip-button bg-primary font-bebas text-black text-lg tracking-[0.22em] uppercase leading-none shadow-[4px_4px_0px_rgba(255,0,0,0.45)] hover:bg-[var(--accent-red-hover)] hover:shadow-[6px_6px_0px_rgba(255,0,0,0.55)] hover:-translate-x-0.5 hover:-translate-y-0.5 active:bg-[var(--accent-red-active)] active:shadow-none active:translate-x-0.5 active:translate-y-0.5",
        outline:
          "p5-clip-button border border-white/20 bg-transparent font-mono text-xs tracking-[0.14em] uppercase text-muted-foreground hover:bg-white/5 hover:text-foreground hover:border-white/35 hover:shadow-[3px_3px_0px_rgba(255,0,0,0.25)] hover:-translate-x-0.5 hover:-translate-y-0.5 active:text-primary active:border-primary/60 active:shadow-none active:translate-x-0.5 active:translate-y-0.5",
        secondary:
          "p5-clip-button border border-white/20 bg-transparent font-mono text-xs tracking-[0.14em] uppercase text-muted-foreground hover:bg-white/5 hover:text-foreground hover:border-white/35 hover:shadow-[3px_3px_0px_rgba(255,0,0,0.25)] hover:-translate-x-0.5 hover:-translate-y-0.5 active:text-primary active:border-primary/60 active:shadow-none active:translate-x-0.5 active:translate-y-0.5",
        ghost:
          "rounded-sm bg-transparent font-mono text-xs tracking-[0.12em] uppercase text-muted-foreground hover:bg-white/5 hover:text-foreground active:bg-primary/10 active:text-primary",
        destructive:
          "p5-clip-button bg-destructive font-bebas text-white text-lg tracking-[0.22em] uppercase leading-none shadow-[4px_4px_0px_rgba(255,0,0,0.35)] hover:bg-destructive/90 hover:shadow-[6px_6px_0px_rgba(255,0,0,0.45)] hover:-translate-x-0.5 hover:-translate-y-0.5 active:shadow-none active:translate-x-0.5 active:translate-y-0.5",
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-10 px-6",
        sm: "h-8 rounded-md px-4 text-[13px]",
        lg: "h-12 px-8 text-base",
        icon: "h-12 w-12 rounded-sm tracking-normal",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

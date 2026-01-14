import * as React from "react";
import { motion, useMotionValue, useSpring, useTransform, useReducedMotion } from "framer-motion";
import { cn } from "@/lib/utils";

export type CountUpProps = {
  value: number;
  className?: string;
  format?: (value: number) => string;
};

export function CountUp({ value, className, format }: CountUpProps) {
  const reducedMotion = useReducedMotion();
  const motionValue = useMotionValue(0);
  const spring = useSpring(motionValue, {
    stiffness: 200,
    damping: 30,
    mass: 0.8,
  });

  const formatted = useTransform(spring, (latest) => {
    const next = typeof format === "function" ? format(latest) : String(Math.round(latest));
    return next;
  });

  React.useEffect(() => {
    if (reducedMotion) return;
    motionValue.set(value);
  }, [motionValue, reducedMotion, value]);

  if (reducedMotion) {
    return (
      <span className={cn("tabular-nums", className)}>
        {typeof format === "function" ? format(value) : value}
      </span>
    );
  }

  return (
    <motion.span className={cn("tabular-nums", className)}>
      {formatted}
    </motion.span>
  );
}


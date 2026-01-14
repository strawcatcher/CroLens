import * as React from "react";

export type PageHeaderProps = {
  title: string;
  description?: string;
  right?: React.ReactNode;
};

export function PageHeader({ title, description, right }: PageHeaderProps) {
  return (
    <div className="flex flex-wrap items-start justify-between gap-4">
      <div className="min-w-0">
        <div className="relative inline-flex max-w-full items-center">
          <div
            className="absolute inset-0 bg-primary p5-clip-button shadow-[4px_4px_0px_rgba(255,255,255,0.18)]"
            aria-hidden="true"
          />
          <h1 className="relative max-w-full truncate px-6 py-2 font-bebas text-3xl tracking-[0.3em] text-white uppercase md:text-4xl">
            {title}
          </h1>
        </div>
        {description ? (
          <div className="mt-3 flex items-center gap-2 text-sm text-muted-foreground">
            <span className="text-primary" aria-hidden="true">
              ◆
            </span>
            <span className="min-w-0">{description}</span>
          </div>
        ) : null}
      </div>

      {right ? <div className="mt-2">{right}</div> : null}
    </div>
  );
}

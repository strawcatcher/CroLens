import * as React from "react";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { cn } from "@/lib/utils";

const crolensPrismTheme: Record<string, React.CSSProperties> = {
  'code[class*="language-"]': {
    color: "var(--text-primary)",
    fontFamily:
      '"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
    fontSize: "13px",
    lineHeight: "1.5",
  },
  'pre[class*="language-"]': {
    color: "var(--text-primary)",
    fontFamily:
      '"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
    fontSize: "13px",
    lineHeight: "1.5",
  },
  comment: { color: "var(--text-muted)" },
  punctuation: { color: "var(--text-secondary)" },
  property: { color: "var(--code-key)" },
  tag: { color: "var(--code-key)" },
  boolean: { color: "var(--code-boolean)" },
  number: { color: "var(--code-number)" },
  string: { color: "var(--code-string)" },
  operator: { color: "var(--text-secondary)" },
  keyword: { color: "var(--code-key)" },
  function: { color: "var(--info)" },
};

export type CodeBlockProps = {
  code: string;
  language?: string;
  showLineNumbers?: boolean;
  className?: string;
  "aria-label"?: string;
};

export function CodeBlock({
  code,
  language = "json",
  showLineNumbers = true,
  className,
  "aria-label": ariaLabel = "Code block",
}: CodeBlockProps) {
  return (
    <div
      className={cn(
        "crolens-codeblock overflow-x-auto rounded-md border border-border bg-[var(--code-bg)]",
        className,
      )}
      aria-label={ariaLabel}
    >
      <SyntaxHighlighter
        language={language}
        style={crolensPrismTheme}
        showLineNumbers={showLineNumbers}
        wrapLongLines
        customStyle={{
          margin: 0,
          background: "transparent",
          padding: 16,
        }}
        lineNumberStyle={{
          minWidth: "32px",
          paddingRight: "12px",
          marginRight: "12px",
          textAlign: "right",
          color: "var(--text-muted)",
          userSelect: "none",
          borderRight: "1px solid var(--border-default)",
        }}
        codeTagProps={{
          style: {
            fontFamily:
              '"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
          },
        }}
      >
        {code}
      </SyntaxHighlighter>
    </div>
  );
}


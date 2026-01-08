"use client";

import { CSSProperties, ReactElement, useMemo } from "react";
import SyntaxHighlighter from "react-syntax-highlighter/dist/esm/prism";
import {
  vscDarkPlus,
  vs,
} from "react-syntax-highlighter/dist/esm/styles/prism";
import cn from "classnames";
import { useTheme } from "@/theme";

function stripBackgroundColor(style: { [key: string]: CSSProperties }): {
  [key: string]: CSSProperties;
} {
  const clone: { [key: string]: CSSProperties } = {};
  for (const key of Object.keys(style)) {
    const entry = { ...style[key] };
    if ("background" in entry) delete entry.background;
    if ("backgroundColor" in entry) delete entry.backgroundColor;
    clone[key] = entry;
  }
  return clone;
}

export function JsonBox({
  json,
  pretty,
  size = "sm",
  className,
}: {
  json: unknown;
  pretty?: boolean;
  size?: "sm" | "md" | "lg";
  className?: string;
}): ReactElement {
  const { theme = "light" } = useTheme();
  const style = useMemo(() => {
    return stripBackgroundColor(theme === "light" ? vs : vscDarkPlus);
  }, [theme]);
  const jsonString = useMemo(() => {
    if (pretty) {
      return JSON.stringify(json, null, 2);
    } else {
      return JSON.stringify(json);
    }
  }, [json, pretty]);

  const [customStyle, codeTagProps] = useMemo(() => {
    if (size === "sm") {
      return [
        {
          margin: 0,
          padding: 0,
          fontSize: "0.875rem",
          lineHeight: "calc(1.25 / 0.875)",
          width: "fit-content",
          border: "none",
          fontFamily: "var(--font-mono)",
        },
        {
          style: {
            fontSize: "0.875rem",
            lineHeight: "calc(1.25 / 0.875)",
            whiteSpace: "pre-wrap",
            fontFamily: "var(--font-mono)",
          },
        },
      ];
    } else if (size === "md") {
      return [
        {
          margin: 0,
          padding: 0,
          fontSize: "1.125rem",
          lineHeight: "calc(1.5 / 0.875)",
          width: "fit-content",
          border: "none",
          fontFamily: "var(--font-mono)",
        },
        {
          style: {
            fontSize: "1.125rem",
            lineHeight: "calc(1.5 / 0.875)",
            whiteSpace: "pre-wrap",
            fontFamily: "var(--font-mono)",
          },
        },
      ];
    } else if (size === "lg") {
      return [
        {
          margin: 0,
          padding: 0,
          fontSize: "1.375rem",
          lineHeight: "calc(1.75 / 0.875)",
          width: "fit-content",
          border: "none",
          fontFamily: "var(--font-mono)",
        },
        {
          style: {
            fontSize: "1.375rem",
            lineHeight: "calc(1.75 / 0.875)",
            whiteSpace: "pre-wrap",
            fontFamily: "var(--font-mono)",
          },
        },
      ];
    } else {
      throw new Error(`Unknown size: ${size}`);
    }
  }, [size]);

  return (
    <div
      className={cn(
        "p-2",
        "rounded-xl",
        "w-fit",
        "[overflow-wrap:anywhere]",
        "bg-background-code",
        className
      )}
    >
      <SyntaxHighlighter
        language="json"
        style={style}
        customStyle={customStyle}
        codeTagProps={codeTagProps}
      >
        {jsonString}
      </SyntaxHighlighter>
    </div>
  );
}

export function TypescriptBox({
  code,
  className,
}: {
  code: string;
  className?: string;
}): ReactElement {
  const { theme = "light" } = useTheme();
  const style = useMemo(() => {
    return stripBackgroundColor(theme === "light" ? vs : vscDarkPlus);
  }, [theme]);
  return (
    <div
      className={cn(
        "p-2",
        "rounded-xl",
        "w-fit",
        "[overflow-wrap:anywhere]",
        "bg-background-code",
        className
      )}
    >
      <SyntaxHighlighter
        language="typescript"
        style={style}
        customStyle={{
          margin: 0,
          padding: 0,
          // backgroundColor: "var(--color-background-code)",
          fontSize: "0.875rem",
          lineHeight: "calc(1.25 / 0.875)",
          width: "fit-content",
          border: "none",
        }}
        codeTagProps={{
          style: {
            fontSize: "0.875rem",
            lineHeight: "calc(1.25 / 0.875)",
            whiteSpace: "pre-wrap",
          },
        }}
      >
        {code}
      </SyntaxHighlighter>
    </div>
  );
}

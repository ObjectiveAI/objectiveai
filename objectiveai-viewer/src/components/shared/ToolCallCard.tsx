import cn from "classnames";
import { useState, useCallback, useEffect, memo } from "react";
import * as Collapsible from "@radix-ui/react-collapsible";

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(t);
  }, [copied]);

  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
    } catch { /* clipboard denied */ }
  }, [text]);

  return (
    <button
      onClick={copy}
      className={cn("text-[10px]", "transition-colors", copied ? cn("text-success") : cn("text-info-dim", "hover:text-copper-bright"))}
      title="Copy to clipboard"
    >
      {copied ? "copied" : "copy"}
    </button>
  );
}

export const ToolCallCard = memo(function ToolCallCard({ call }: { call: { id?: string | null; function?: { name?: string | null; arguments?: string | null } | null } }) {
  const fn = call.function;
  let formattedArgs = fn?.arguments ?? "";
  try {
    formattedArgs = JSON.stringify(JSON.parse(formattedArgs), null, 2);
  } catch { /* keep raw */ }

  return (
    <div className={cn("bg-ground-surface", "border", "border-node-border", "rounded-md", "p-2.5", "text-xs")}>
      <div className={cn("flex", "items-center", "gap-2", "mb-1")}>
        <span className={cn("font-semibold", "font-mono", "text-copper-mid")}>{fn?.name ?? "unknown"}</span>
        {formattedArgs && <CopyButton text={formattedArgs} />}
      </div>
      {formattedArgs && (
        <Collapsible.Root defaultOpen={formattedArgs.length < 200}>
          <Collapsible.Trigger className={cn("text-[10px]", "text-info-dim", "cursor-pointer", "hover:text-info-mid")} aria-label="Toggle function arguments">
            args
          </Collapsible.Trigger>
          <Collapsible.Content>
            <div className={cn("font-mono", "text-[11px]", "whitespace-pre-wrap", "break-words", "text-info-mid", "max-h-[150px]", "overflow-y-auto", "mt-1")}>
              {formattedArgs}
            </div>
          </Collapsible.Content>
        </Collapsible.Root>
      )}
    </div>
  );
});

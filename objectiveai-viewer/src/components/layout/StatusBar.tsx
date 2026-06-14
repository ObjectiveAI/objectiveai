import { useState, useEffect } from "react";
import cn from "classnames";
import type { Entry } from "../../types";
import { useElapsedTime } from "../../hooks/useElapsedTime";
import { formatCost } from "../../lib/format";
import { isLastAssistantDone } from "../../lib/typeGuards";
import { getDroppedEventCount, onDroppedCountChange } from "../../classify";

function useDroppedCount(): number {
  const [count, setCount] = useState(getDroppedEventCount);
  useEffect(() => onDroppedCountChange(setCount), []);
  return count;
}

export function StatusBar({ entries, isHistorical }: { entries: Entry[]; isHistorical?: boolean }) {
  const droppedCount = useDroppedCount();
  if (entries.length === 0) {
    return (
      <footer className={cn("flex", "items-center", "gap-4", "px-6", "py-2", "border-t", "border-node-border", "bg-ground-raised", "font-mono", "text-[10px]", "text-info-dim", "tabular-nums", "select-none")}>
        <div className={cn("flex", "items-center", "gap-1.5")}>
          <div className={cn("w-1.5", "h-1.5", "rounded-full", "bg-success")} />
          <span>Ready</span>
        </div>
        <span className={cn("text-info-dim/70")}>localhost:5001</span>
      </footer>
    );
  }

  const activeCount = isHistorical ? 0 : entries.filter((e) => {
    if (e.error) return false;
    if (!e.chunk) return true;
    if ((e.chunk as { usage?: unknown }).usage != null) return false;
    switch (e.kind) {
      case "agent-completion":
        return !isLastAssistantDone(e.chunk.messages);
      case "execution":
        return e.chunk.output == null;
    }
  }).length;

  const earliestActiveReceived = entries.reduce<number | null>((min, e) => {
    if (e.error) return min;
    if (!e.chunk) return min === null ? e.receivedAt : Math.min(min, e.receivedAt);
    if ((e.chunk as { usage?: unknown }).usage != null) return min;
    return min === null ? e.receivedAt : Math.min(min, e.receivedAt);
  }, null);

  const elapsed = useElapsedTime(activeCount > 0 ? earliestActiveReceived : null);

  let totalTokens = 0;
  let totalCost = 0;
  for (const e of entries) {
    const usage = e.chunk && "usage" in e.chunk ? (e.chunk as { usage?: { total_tokens?: number; cost?: number } }).usage : undefined;
    if (usage) {
      totalTokens += usage.total_tokens ?? 0;
      totalCost += typeof usage.cost === "number" ? usage.cost : 0;
    }
  }

  return (
    <footer role="status" aria-live="polite" className={cn("flex", "items-center", "gap-4", "px-4", "py-2", "border-t", "border-node-border", "bg-ground-raised", "font-mono", "text-[10px]", "text-info-dim", "tabular-nums", "select-none", "overflow-hidden", "whitespace-nowrap", "min-w-0")}>
      <div className={cn("flex", "items-center", "gap-1.5", "shrink-0")}>
        {isHistorical ? (
          <>
            <div className={cn("w-1.5", "h-1.5", "rounded-full", "bg-info-dim")} />
            <span>Historical</span>
          </>
        ) : (
          <>
            <div className={cn("w-1.5", "h-1.5", "rounded-full", activeCount > 0 ? cn("bg-copper-hot", "animate-pulse") : "bg-info-dim")} />
            <span>{activeCount} active</span>
          </>
        )}
      </div>
      {!isHistorical && activeCount > 0 && <span className={cn("shrink-0")}>{elapsed}</span>}
      {totalTokens > 0 && <span className={cn("shrink-0")}>{totalTokens.toLocaleString()} tokens</span>}
      {totalCost > 0 && <span className={cn("shrink-0")}>{formatCost(totalCost)}</span>}
      {droppedCount > 0 && <span className={cn("text-error", "shrink-0")}>{droppedCount} dropped</span>}
      <span className={cn("ml-auto", "shrink-0")}>{entries.length} total</span>
    </footer>
  );
}

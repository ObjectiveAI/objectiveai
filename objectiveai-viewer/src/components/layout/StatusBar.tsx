import cn from "classnames";
import type { Entry } from "../../types";
import { formatCost } from "../../lib/format";
import { useActiveAgents } from "../../hooks/useActiveAgents";
import type { ListenerStream } from "../../hooks/useListener";

export function StatusBar({ entries, isHistorical, stream }: { entries: Entry[]; isHistorical?: boolean; stream: ListenerStream }) {
  const activeAgents = useActiveAgents(stream);
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
        <div className={cn("w-1.5", "h-1.5", "rounded-full", activeAgents.length > 0 ? cn("bg-copper-hot", "animate-pulse") : "bg-info-dim")} />
        <span>{activeAgents.length} active {activeAgents.length === 1 ? "agent" : "agents"}</span>
      </div>
      {isHistorical && (
        <div className={cn("flex", "items-center", "gap-1.5", "shrink-0")}>
          <div className={cn("w-1.5", "h-1.5", "rounded-full", "bg-info-dim")} />
          <span>Historical</span>
        </div>
      )}
      {totalTokens > 0 && <span className={cn("shrink-0")}>{totalTokens.toLocaleString()} tokens</span>}
      {totalCost > 0 && <span className={cn("shrink-0")}>{formatCost(totalCost)}</span>}
    </footer>
  );
}

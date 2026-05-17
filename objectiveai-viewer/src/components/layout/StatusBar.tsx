import type { Entry } from "../../types";
import { useElapsedTime } from "../../hooks/useElapsedTime";
import { formatCost } from "../../lib/format";
import { isLastAssistantDone } from "../../lib/typeGuards";

export function StatusBar({ entries, isHistorical }: { entries: Entry[]; isHistorical?: boolean }) {
  if (entries.length === 0) {
    return (
      <footer className="flex items-center gap-4 px-6 py-2 border-t border-node-border bg-ground-raised font-mono text-[10px] text-info-dim tabular-nums select-none">
        <div className="flex items-center gap-1.5">
          <div className="w-1.5 h-1.5 rounded-full bg-success" />
          <span>Ready</span>
        </div>
        <span className="text-info-dim/70">localhost:5001</span>
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
      case "invention":
        return e.chunk.inventions.length === 0 || !e.chunk.inventions.every(
          (inv: { function?: unknown; error?: unknown }) => inv.function != null || inv.error != null
        );
      case "laboratory":
        return e.chunk.evaluations.length === 0 || !e.chunk.evaluations.every(
          (ev: { output?: unknown }) => ev.output != null
        );
    }
  }).length;

  const firstCreated = entries.reduce<number | null>((min, e) => {
    const c = e.chunk && "created" in e.chunk ? (e.chunk as { created?: number }).created : undefined;
    if (c == null) return min;
    const ms = c * 1000;
    return min === null ? ms : Math.min(min, ms);
  }, null);

  const elapsed = useElapsedTime(activeCount > 0 ? firstCreated : null);

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
    <footer className="flex items-center gap-6 px-6 py-2 border-t border-node-border bg-ground-raised font-mono text-[10px] text-info-dim tabular-nums select-none">
      <div className="flex items-center gap-1.5">
        {isHistorical ? (
          <>
            <div className="w-1.5 h-1.5 rounded-full bg-info-dim" />
            <span>Historical</span>
          </>
        ) : (
          <>
            <div className={`w-1.5 h-1.5 rounded-full ${activeCount > 0 ? "bg-copper-hot animate-pulse" : "bg-info-dim"}`} />
            <span>{activeCount} active</span>
          </>
        )}
      </div>
      {!isHistorical && activeCount > 0 && <span>{elapsed}</span>}
      {totalTokens > 0 && <span>{totalTokens.toLocaleString()} tokens</span>}
      {totalCost > 0 && <span>{formatCost(totalCost)}</span>}
      <span className="ml-auto">{entries.length} total</span>
    </footer>
  );
}

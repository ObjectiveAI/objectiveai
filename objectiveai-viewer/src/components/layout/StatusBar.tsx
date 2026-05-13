import type { Entry } from "../../types";
import { useElapsedTime } from "../../hooks/useElapsedTime";

export function StatusBar({ entries }: { entries: Entry[] }) {
  if (entries.length === 0) return null;

  const activeCount = entries.filter((e) => {
    if (e.error) return false;
    if (!e.chunk) return true;
    switch (e.kind) {
      case "agent-completion":
        return !e.chunk.messages.some(
          (m: { role: string; finish_reason?: string | null }) =>
            m.role === "assistant" && m.finish_reason
        );
      case "execution":
        return !("output" in e.chunk && e.chunk.output != null);
      case "invention":
        return !e.chunk.inventions.every(
          (inv: { function?: unknown }) => inv.function != null
        );
      case "laboratory":
        return !e.chunk.evaluations.every(
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
    <footer className="flex items-center gap-6 px-6 py-2 border-t border-node-border bg-ground-raised font-mono text-[10px] text-info-dim tabular-nums">
      <div className="flex items-center gap-1.5">
        <div className={`w-1.5 h-1.5 rounded-full ${activeCount > 0 ? "bg-copper-hot animate-pulse" : "bg-info-dim"}`} />
        <span>{activeCount} active</span>
      </div>
      {activeCount > 0 && <span>{elapsed}</span>}
      {totalTokens > 0 && <span>{totalTokens.toLocaleString()} tokens</span>}
      {totalCost > 0 && <span>${totalCost.toFixed(6)}</span>}
      <span className="ml-auto">{entries.length} total</span>
    </footer>
  );
}

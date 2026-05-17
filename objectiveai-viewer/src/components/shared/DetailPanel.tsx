import { useMemo } from "react";
import type { Entry } from "../../types";

function timingBreakdown(entry: Entry): { label: string; value: string }[] {
  const rows: { label: string; value: string }[] = [];
  const chunk = entry.chunk as Record<string, unknown> | null;
  if (!chunk) return rows;

  const created = chunk.created as number | undefined;
  if (created) {
    rows.push({ label: "Created", value: new Date(created * 1000).toLocaleString() });
  }

  const usage = chunk.usage as { prompt_tokens?: number; completion_tokens?: number; total_tokens?: number } | undefined;
  if (usage) {
    if (usage.prompt_tokens != null) rows.push({ label: "Prompt tokens", value: usage.prompt_tokens.toLocaleString() });
    if (usage.completion_tokens != null) rows.push({ label: "Completion tokens", value: usage.completion_tokens.toLocaleString() });
    if (usage.total_tokens != null) rows.push({ label: "Total tokens", value: usage.total_tokens.toLocaleString() });
  }

  const model = chunk.model as string | undefined;
  if (model) rows.push({ label: "Model", value: model });

  const obj = chunk.object as string | undefined;
  if (obj) rows.push({ label: "Object", value: obj });

  return rows;
}

export function DetailPanel({ entry, onClose }: { entry: Entry; onClose: () => void }) {
  const timing = useMemo(() => timingBreakdown(entry), [entry.chunk]);
  const requestJson = useMemo(() => JSON.stringify(entry.request, null, 2), [entry.request]);
  const chunkJson = useMemo(() => entry.chunk ? JSON.stringify(entry.chunk, null, 2) : null, [entry.chunk]);

  return (
    <div className="flex flex-col h-full bg-ground-raised border-l border-node-border">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-node-border shrink-0">
        <span className="text-[10px] font-mono uppercase tracking-widest text-info-dim">Detail</span>
        <span className="text-[10px] font-mono text-info-dim/50 truncate ml-1">{entry.id.slice(0, 12)}</span>
        <button
          onClick={onClose}
          aria-label="Close detail panel"
          className="ml-auto p-1 rounded-sm text-info-dim hover:text-info-bright hover:bg-ground-surface transition-colors"
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <path d="M2 2L8 8M8 2L2 8" />
          </svg>
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-3 py-3 space-y-4">
        {timing.length > 0 && (
          <section>
            <h3 className="text-[9px] font-mono uppercase tracking-widest text-info-dim mb-2">Timing</h3>
            <dl className="space-y-1">
              {timing.map((row) => (
                <div key={row.label} className="flex items-baseline justify-between gap-2">
                  <dt className="text-[10px] text-info-dim shrink-0">{row.label}</dt>
                  <dd className="text-[10px] font-mono text-info-mid text-right truncate">{row.value}</dd>
                </div>
              ))}
            </dl>
          </section>
        )}

        <section>
          <h3 className="text-[9px] font-mono uppercase tracking-widest text-info-dim mb-2">Request</h3>
          <pre className="text-[10px] font-mono text-info-mid bg-ground p-2.5 rounded-sm border border-node-border overflow-x-auto whitespace-pre-wrap break-words max-h-60 overflow-y-auto">
            {requestJson}
          </pre>
        </section>

        {chunkJson && (
          <section>
            <h3 className="text-[9px] font-mono uppercase tracking-widest text-info-dim mb-2">Response</h3>
            <pre className="text-[10px] font-mono text-info-mid bg-ground p-2.5 rounded-sm border border-node-border overflow-x-auto whitespace-pre-wrap break-words max-h-96 overflow-y-auto">
              {chunkJson}
            </pre>
          </section>
        )}

        {entry.error && (
          <section>
            <h3 className="text-[9px] font-mono uppercase tracking-widest text-error mb-2">Error</h3>
            <pre className="text-[10px] font-mono text-error/80 bg-error/5 p-2.5 rounded-sm border border-error/20 whitespace-pre-wrap break-words">
              {JSON.stringify(entry.error, null, 2)}
            </pre>
          </section>
        )}
      </div>
    </div>
  );
}

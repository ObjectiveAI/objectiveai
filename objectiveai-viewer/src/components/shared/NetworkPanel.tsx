import { useState } from "react";
import type { ApiCallEntry } from "../../hooks/useApiCalls";

function formatDuration(entry: ApiCallEntry): string {
  if (!entry.endedAt) {
    const elapsed = Date.now() - entry.startedAt;
    return `${elapsed}ms…`;
  }
  return `${entry.endedAt - entry.startedAt}ms`;
}

function StatusDot({ status }: { status: ApiCallEntry["status"] }) {
  const color =
    status === "streaming"
      ? "bg-copper-bright animate-pulse"
      : status === "complete"
        ? "bg-green-500"
        : "bg-error";
  return <span className={`inline-block w-1.5 h-1.5 rounded-full ${color}`} />;
}

export function NetworkPanel({ entries }: { entries: ApiCallEntry[] }) {
  const [expanded, setExpanded] = useState(false);

  if (entries.length === 0) return null;

  return (
    <div className="border-t border-node-border bg-ground-surface">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-2 px-3 py-1.5 text-[10px] font-mono text-info-dim hover:text-info-mid transition-colors select-none"
      >
        <svg
          width="8"
          height="8"
          viewBox="0 0 8 8"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          className={`transition-transform ${expanded ? "rotate-90" : ""}`}
        >
          <path d="M2 1L6 4L2 7" />
        </svg>
        <span>Network</span>
        <span className="text-info-dim/70">({entries.length})</span>
        {entries.some((e) => e.status === "streaming") && (
          <span className="ml-auto text-copper-bright">streaming</span>
        )}
      </button>

      {expanded && (
        <div className="max-h-48 overflow-y-auto border-t border-node-border">
          <table className="w-full text-[10px] font-mono">
            <thead>
              <tr className="text-info-dim border-b border-node-border/50">
                <th className="text-left px-3 py-1 font-normal"></th>
                <th className="text-left px-3 py-1 font-normal">Endpoint</th>
                <th className="text-right px-3 py-1 font-normal">Chunks</th>
                <th className="text-right px-3 py-1 font-normal">Duration</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((entry) => (
                <tr
                  key={entry.id}
                  className={`border-b border-node-border/30 ${
                    entry.status === "error" ? "bg-error/5" : ""
                  }`}
                >
                  <td className="px-3 py-1">
                    <StatusDot status={entry.status} />
                  </td>
                  <td className="px-3 py-1 text-info-mid">
                    <span className="text-info-dim mr-1.5">{entry.method}</span>
                    {entry.path}
                  </td>
                  <td className="px-3 py-1 text-right text-info-dim">
                    {entry.chunkCount}
                  </td>
                  <td className="px-3 py-1 text-right text-info-dim">
                    {formatDuration(entry)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

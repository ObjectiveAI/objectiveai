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

function formatError(error: unknown): string {
  if (error === null || error === undefined) return "";
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error, null, 2);
  } catch {
    return String(error);
  }
}

export function NetworkPanel({ entries }: { entries: ApiCallEntry[] }) {
  const [expanded, setExpanded] = useState(false);
  const [expandedRow, setExpandedRow] = useState<string | null>(null);

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
        {entries.some((e) => e.status === "error") && (
          <span className="ml-1 text-error">{entries.filter((e) => e.status === "error").length} errors</span>
        )}
        {entries.some((e) => e.status === "streaming") && (
          <span className="ml-auto text-copper-bright">streaming</span>
        )}
      </button>

      {expanded && (
        <div className="max-h-64 overflow-y-auto border-t border-node-border">
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
                <>
                  <tr
                    key={entry.id}
                    onClick={() => setExpandedRow(expandedRow === entry.id ? null : entry.id)}
                    className={`border-b border-node-border/30 cursor-pointer hover:bg-ground-raised/50 ${
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
                  {expandedRow === entry.id && (
                    <tr key={`${entry.id}-detail`}>
                      <td colSpan={4} className="px-3 py-2 bg-ground-raised/30">
                        <div className="space-y-1">
                          <div className="text-info-dim">
                            <span className="text-info-dim/70">Started:</span>{" "}
                            {new Date(entry.startedAt).toLocaleTimeString()}
                          </div>
                          {entry.endedAt && (
                            <div className="text-info-dim">
                              <span className="text-info-dim/70">Ended:</span>{" "}
                              {new Date(entry.endedAt).toLocaleTimeString()}
                            </div>
                          )}
                          {entry.error != null && (
                            <div className="mt-1">
                              <div className="text-error/70 text-[9px] uppercase tracking-wide mb-0.5">Error</div>
                              <pre className="text-error text-[10px] whitespace-pre-wrap break-words max-h-32 overflow-y-auto bg-error/5 rounded-sm px-2 py-1">
                                {formatError(entry.error)}
                              </pre>
                            </div>
                          )}
                        </div>
                      </td>
                    </tr>
                  )}
                </>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

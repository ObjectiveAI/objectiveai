import { useEffect, useState } from "react";
import cn from "classnames";
import { tauriListen } from "../lib/tauri";
import { logsSnapshot, type LogEntry } from "../lib/logs";
import { useAgo } from "../hooks/useAgo";

/** Client-side cap: the Rust ring holds 1000, but live upserts can
 * accumulate past whatever the boot snapshot carried — trim from the
 * oldest seq once comfortably past the server cap. */
const CLIENT_CAP = 1200;

/** Levels rendered as failures (error accent + emphasized message). */
const ERROR_LEVELS = new Set(["error", "uncaught", "unhandledrejection"]);

/** The viewer-logs home tab: everything the capture initialization
 * script hoovered out of every webview, newest first. Pure view over
 * the Rust-side ring — subscribe FIRST, then snapshot, upsert both by
 * `seq` (a coalesced repeat re-broadcasts its seq with a bumped
 * count; on a seq collision the higher count wins, so a stale
 * snapshot can never roll an entry back). */
export function ViewerLogsPane() {
  const [entries, setEntries] = useState<ReadonlyMap<number, LogEntry>>(
    new Map(),
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const upsert = (incoming: LogEntry[]) => {
      setEntries((prev) => {
        const next = new Map(prev);
        for (const entry of incoming) {
          const have = next.get(entry.seq);
          if (!have || entry.count >= have.count) {
            next.set(entry.seq, entry);
          }
        }
        if (next.size > CLIENT_CAP) {
          const excess = [...next.keys()].sort((a, b) => a - b);
          for (const seq of excess.slice(0, next.size - CLIENT_CAP)) {
            next.delete(seq);
          }
        }
        return next;
      });
    };
    void (async () => {
      unlisten = await tauriListen<LogEntry>("logs://appended", (e) => {
        if (!disposed) upsert([e.payload]);
      });
      if (disposed) {
        unlisten?.();
        return;
      }
      const snapshot = await logsSnapshot();
      if (snapshot && !disposed) upsert(snapshot);
    })();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const list = [...entries.values()].sort((a, b) => b.seq - a.seq);

  return (
    <div className={cn("flex-1", "min-h-0", "overflow-y-auto")}>
      {list.length === 0 && (
        <div
          className={cn(
            "p-6",
            "font-mono",
            "text-[11px]",
            "text-info-dim",
            "select-none",
          )}
        >
          nothing captured yet
        </div>
      )}
      {list.map((entry) => (
        <LogRow key={entry.seq} entry={entry} />
      ))}
    </div>
  );
}

/** One entry. A separate component because `useAgo` is a hook (can't
 * run inside the map). */
function LogRow({ entry }: { entry: LogEntry }) {
  const at = new Date(entry.at_ms);
  const ago = useAgo(at.toISOString());
  const isError = ERROR_LEVELS.has(entry.level);
  const isWarn = entry.level === "warn";
  return (
    <div
      data-log-row
      className={cn(
        "px-4",
        "py-1.5",
        "border-b",
        "border-node-border",
        "font-mono",
        "text-[11px]",
        isError && "bg-error/5",
      )}
    >
      <div className={cn("flex", "items-baseline", "gap-3", "min-w-0")}>
        <span
          data-log-level
          className={cn(
            "shrink-0",
            "w-24",
            "uppercase",
            "text-[9px]",
            "tracking-wider",
            isError ? "text-error" : isWarn ? "text-copper-hot" : "text-info-dim",
          )}
        >
          {entry.level}
        </span>
        <span
          data-log-source
          className={cn("shrink-0", "text-copper-dim", "truncate", "max-w-48")}
        >
          {entry.source}
        </span>
        <span
          data-log-message
          className={cn(
            "min-w-0",
            "break-words",
            "whitespace-pre-wrap",
            isError ? "text-info-full" : "text-info-mid",
          )}
        >
          {entry.message}
        </span>
        {entry.count > 1 && (
          <span
            data-log-count
            title="consecutive identical reports"
            className={cn(
              "shrink-0",
              "px-1",
              "rounded-xs",
              "bg-ground-surface",
              "text-info-bright",
              "tabular-nums",
            )}
          >
            ×{entry.count}
          </span>
        )}
        <span
          data-log-at
          title={at.toLocaleString()}
          className={cn(
            "ml-auto",
            "shrink-0",
            "text-info-dim",
            "tabular-nums",
            "whitespace-nowrap",
          )}
        >
          {at.toLocaleTimeString()} · {ago}
        </span>
      </div>
      {entry.detail !== null && (
        <pre
          data-log-detail
          className={cn(
            "mt-1",
            "ml-27",
            "overflow-x-auto",
            "text-[10px]",
            "text-info-dim",
            "whitespace-pre-wrap",
          )}
        >
          {entry.detail}
        </pre>
      )}
    </div>
  );
}

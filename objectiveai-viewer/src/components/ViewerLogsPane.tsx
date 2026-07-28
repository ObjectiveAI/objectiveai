import { useEffect, useState } from "react";
import cn from "classnames";
import { tauriListen } from "../lib/tauri";
import { logsPull, type LogEntry } from "../lib/logs";
import { useAgo } from "../hooks/useAgo";
import { useBottomTether } from "../hooks/useBottomTether";

/** How much history one pull asks for (the Rust side caps at 1000). */
const PULL_COUNT = 1000;

/** The JS-side ring: Rust streams from disk and holds no history, so
 * the memory bound lives HERE — once past the cap, the oldest seqs
 * fall off. */
const CLIENT_CAP = 1200;

/** Levels rendered as failures (error accent + emphasized message). */
const ERROR_LEVELS = new Set(["error", "uncaught", "unhandledrejection"]);

/** The viewer-logs home tab: everything the capture initialization
 * script hoovered out of every webview, oldest first. A pure view
 * over the Rust-side logfile — subscribe FIRST (live appends), then
 * pull history (streamed newest-first off disk, i.e. prepends); the
 * two flows interleave safely because everything keys by `seq` and
 * inserts are idempotent. */
export function ViewerLogsPane() {
  const [entries, setEntries] = useState<ReadonlyMap<number, LogEntry>>(
    new Map(),
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const insert = (entry: LogEntry) => {
      if (disposed) return;
      setEntries((prev) => {
        if (prev.has(entry.seq)) return prev;
        const next = new Map(prev);
        next.set(entry.seq, entry);
        if (next.size > CLIENT_CAP) {
          const seqs = [...next.keys()].sort((a, b) => a - b);
          for (const seq of seqs.slice(0, next.size - CLIENT_CAP)) {
            next.delete(seq);
          }
        }
        return next;
      });
    };
    void (async () => {
      unlisten = await tauriListen<LogEntry>("logs://appended", (e) => {
        insert(e.payload);
      });
      if (disposed) {
        unlisten?.();
        return;
      }
      await logsPull(PULL_COUNT, insert);
    })();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Bottom-tether: appends keep the scroller at the bottom while it
  // IS at the bottom; scrolling up releases it.
  const { ref, onScroll } = useBottomTether(entries);

  // Oldest first, newest at the end — plain document flow (short
  // content sits at the TOP). Consecutive identical (source, level,
  // message) entries merge into one DISPLAY row with a ×count —
  // render-time coalescing; the store keeps every entry distinct.
  const rows: { entry: LogEntry; count: number }[] = [];
  for (const entry of [...entries.values()].sort((a, b) => a.seq - b.seq)) {
    const last = rows[rows.length - 1];
    if (
      last !== undefined &&
      last.entry.source === entry.source &&
      last.entry.level === entry.level &&
      last.entry.message === entry.message
    ) {
      last.count += 1;
      if (last.entry.detail === null && entry.detail !== null) {
        last.entry = { ...last.entry, detail: entry.detail };
      }
    } else {
      rows.push({ entry, count: 1 });
    }
  }

  return (
    <div
      ref={ref}
      onScroll={onScroll}
      className={cn("flex-1", "min-h-0", "overflow-y-auto")}
    >
      {rows.length === 0 && (
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
      {rows.map((row) => (
        <LogRow key={row.entry.seq} entry={row.entry} count={row.count} />
      ))}
    </div>
  );
}

/** One display row. A separate component because `useAgo` is a hook
 * (can't run inside the map). */
function LogRow({ entry, count }: { entry: LogEntry; count: number }) {
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
        {count > 1 && (
          <span
            data-log-count
            title="consecutive identical entries"
            className={cn(
              "shrink-0",
              "px-1",
              "rounded-xs",
              "bg-ground-surface",
              "text-info-bright",
              "tabular-nums",
            )}
          >
            ×{count}
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

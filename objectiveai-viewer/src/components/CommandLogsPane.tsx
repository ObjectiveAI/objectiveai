import { useEffect, useState } from "react";
import cn from "classnames";
import { tauriListen } from "../lib/tauri";
import {
  commandLogsPull,
  type CommandRequestEntry,
} from "../lib/commandLogs";
import { tabsOpen } from "../lib/tabs";
import { useAgo } from "../hooks/useAgo";
import { useBottomTether } from "../hooks/useBottomTether";

/** How much history one pull asks for (the Rust side caps at 1000). */
const PULL_COUNT = 1000;

/** The JS-side ring — oldest seqs fall off past the cap. */
const CLIENT_CAP = 1200;

/** The command-logs home tab: one row per command run captured off
 * the daemon's /listen broadcast, WITH the producer's identity —
 * who ran it is the point. Clicking a row opens that request's own
 * tab (its response items stream there). Same view contract as
 * viewer-logs: subscribe FIRST (live appends), then pull history
 * (streamed newest-first, i.e. prepends); keyed by broadcast id,
 * ordered by seq. */
export function CommandLogsPane() {
  const [entries, setEntries] = useState<
    ReadonlyMap<string, CommandRequestEntry>
  >(new Map());

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const insert = (entry: CommandRequestEntry) => {
      if (disposed) return;
      setEntries((prev) => {
        if (prev.has(entry.id)) return prev;
        const next = new Map(prev);
        next.set(entry.id, entry);
        if (next.size > CLIENT_CAP) {
          const bySeq = [...next.values()].sort((a, b) => a.seq - b.seq);
          for (const old of bySeq.slice(0, next.size - CLIENT_CAP)) {
            next.delete(old.id);
          }
        }
        return next;
      });
    };
    void (async () => {
      unlisten = await tauriListen<CommandRequestEntry>(
        "command-logs://request",
        (e) => insert(e.payload),
      );
      if (disposed) {
        unlisten?.();
        return;
      }
      await commandLogsPull(PULL_COUNT, insert);
    })();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Bottom-tether: appends keep the scroller at the bottom while it
  // IS at the bottom; scrolling up releases it.
  const { ref, onScroll } = useBottomTether(entries);

  const list = [...entries.values()].sort((a, b) => a.seq - b.seq);

  return (
    <div
      ref={ref}
      onScroll={onScroll}
      className={cn("flex-1", "min-h-0", "overflow-y-auto")}
    >
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
          no commands captured yet
        </div>
      )}
      {list.map((entry) => (
        <RequestRow key={entry.id} entry={entry} />
      ))}
    </div>
  );
}

/** The producer's identity, compact: the hierarchy (or "anonymous"),
 * the plugin trio when present, a task badge when scheduler-fired. */
function IdentityCell({ entry }: { entry: CommandRequestEntry }) {
  const plugin =
    entry.plugin_owner !== undefined
      ? `${entry.plugin_owner}/${entry.plugin_name}@${entry.plugin_version}`
      : null;
  return (
    <span className={cn("flex", "items-baseline", "gap-2", "min-w-0")}>
      <span className={cn("text-info-mid", "truncate")}>
        {entry.agent_instance_hierarchy ?? "anonymous"}
      </span>
      {plugin !== null && (
        <span
          data-request-plugin
          className={cn("text-copper-dim", "truncate")}
          title="plugin identity"
        >
          {plugin}
        </span>
      )}
      {entry.task && (
        <span
          data-request-task
          title="fired by the task scheduler"
          className={cn(
            "shrink-0",
            "px-1",
            "rounded-xs",
            "bg-ground-surface",
            "text-copper-hot",
            "text-[9px]",
            "uppercase",
            "tracking-wider",
          )}
        >
          task
        </span>
      )}
    </span>
  );
}

/** One captured request. Click = open its items tab. */
function RequestRow({ entry }: { entry: CommandRequestEntry }) {
  const at = new Date(entry.at_ms);
  const ago = useAgo(at.toISOString());
  const path = entry.path ?? "?";
  return (
    <button
      type="button"
      data-request-row
      onClick={() =>
        tabsOpen({ type: "command_log", id: entry.id, path })
      }
      className={cn(
        "block",
        "w-full",
        "text-left",
        "px-4",
        "py-1.5",
        "border-b",
        "border-node-border",
        "font-mono",
        "text-[11px]",
        "cursor-pointer",
        "hover:bg-ground-raised",
      )}
    >
      <div className={cn("flex", "items-baseline", "gap-3", "min-w-0")}>
        <span
          data-request-path
          className={cn("shrink-0", "w-56", "truncate", "text-info-bright")}
        >
          {path}
        </span>
        <IdentityCell entry={entry} />
        <span
          data-request-at
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
    </button>
  );
}

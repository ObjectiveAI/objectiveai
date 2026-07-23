import { useEffect, useState } from "react";
import cn from "classnames";
import { tauriListen } from "../lib/tauri";
import {
  commandLogItemsPull,
  type CommandItemEntry,
  type CommandItemEvent,
} from "../lib/commandLogs";
import { useAgo } from "../hooks/useAgo";

/** How much history one pull asks for (the Rust side caps at 1000). */
const PULL_COUNT = 1000;

/** The JS-side ring — oldest seqs fall off past the cap. */
const CLIENT_CAP = 1200;

/** One captured request's viewer: its response items, oldest first,
 * live-tailing while the run is still streaming. The live event
 * feed is global (`command-logs://item`) — this pane filters by its
 * own request id; history pulls stream that request's items file
 * backwards (prepends), and both key by seq. */
export function CommandLogPane({ id }: { id: string }) {
  const [items, setItems] = useState<ReadonlyMap<number, CommandItemEntry>>(
    new Map(),
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const insert = (item: CommandItemEntry) => {
      if (disposed) return;
      setItems((prev) => {
        if (prev.has(item.seq)) return prev;
        const next = new Map(prev);
        next.set(item.seq, item);
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
      unlisten = await tauriListen<CommandItemEvent>(
        "command-logs://item",
        (e) => {
          if (e.payload.request_id === id) insert(e.payload.item);
        },
      );
      if (disposed) {
        unlisten?.();
        return;
      }
      await commandLogItemsPull(id, PULL_COUNT, insert);
    })();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [id]);

  const list = [...items.values()].sort((a, b) => a.seq - b.seq);

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
          no response items yet
        </div>
      )}
      {list.map((item) => (
        <ItemRow key={item.seq} item={item} />
      ))}
    </div>
  );
}

/** One response item (or the end terminator). */
function ItemRow({ item }: { item: CommandItemEntry }) {
  const at = new Date(item.at_ms);
  const ago = useAgo(at.toISOString());
  return (
    <div
      data-item-row
      className={cn(
        "px-4",
        "py-1.5",
        "border-b",
        "border-node-border",
        "font-mono",
        "text-[11px]",
      )}
    >
      <div className={cn("flex", "items-baseline", "gap-3", "min-w-0")}>
        {item.end === true ? (
          <span
            data-item-end
            className={cn(
              "text-info-dim",
              "uppercase",
              "text-[9px]",
              "tracking-wider",
            )}
          >
            stream ended
          </span>
        ) : (
          <span
            data-item-value
            className={cn(
              "min-w-0",
              "break-words",
              "whitespace-pre-wrap",
              "text-info-mid",
            )}
          >
            {JSON.stringify(item.value)}
          </span>
        )}
        <span
          data-item-at
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
    </div>
  );
}

import { useEffect, useRef, useState } from "react";
import cn from "classnames";
import { tauriListen } from "../lib/tauri";
import {
  tabIconUrl,
  tabsInventory,
  tabsReorder,
  tabsToggle,
  type TabInventoryEntry,
} from "../lib/tabs";
import { IdentityIcon } from "../components/shared/IdentityIcon";

/** The tabs home tab: every tab loaded from the system THIS boot —
 * the root objectiveai tabs and every scanned plugin tab — each with
 * its identity icon, a persisted enabled/disabled toggle, and a
 * persisted ORDER (drag rows to reorder; the config-file order IS
 * the boot order, and outside user-controlled mode the live strip
 * follows too). The tabs tab itself is greyed: permanent, not
 * toggleable, not draggable (but displaceable by other rows'
 * moves).
 *
 * STICKINESS (drag semantics): dragging an ENABLED row carries its
 * trailing run of MIDDLE disabled rows (sandwiched runs belong to
 * the enabled row above them); drops land only at block boundaries,
 * and never before the leading disabled run nor after the trailing
 * one — start/end disabled rows stay pinned. Dragging a DISABLED
 * row moves just itself, anywhere (re-parenting is the point).
 *
 * Pure view over the Rust inventory — subscribe FIRST
 * (`inventory://changed` carries the full ordered list on every
 * change), then pull. During a drag a local overlay previews the
 * order; the commit round-trips through Rust and the next inventory
 * event is truth. */
export default function TabsTab() {
  const [entries, setEntries] = useState<TabInventoryEntry[]>([]);
  const [overlay, setOverlay] = useState<TabInventoryEntry[] | null>(null);
  const gesture = useRef<{
    pointerId: number;
    key: string;
    enabled: boolean;
    startY: number;
    rowHeight: number;
    moved: boolean;
  } | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void (async () => {
      unlisten = await tauriListen<TabInventoryEntry[]>(
        "inventory://changed",
        (e) => {
          if (disposed) return;
          // External truth — replaces everything, and cancels any
          // in-flight gesture (its base just went stale).
          gesture.current = null;
          setEntries(e.payload);
          setOverlay(null);
        },
      );
      if (disposed) {
        unlisten?.();
        return;
      }
      const inventory = await tabsInventory();
      if (inventory && !disposed) setEntries(inventory);
    })();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const display = overlay ?? entries;

  const cancelGesture = () => {
    if (gesture.current) {
      gesture.current = null;
      setOverlay(null);
    }
  };

  const onRowPointerDown = (
    e: React.PointerEvent<HTMLDivElement>,
    entry: TabInventoryEntry,
  ) => {
    if (entry.permanent || e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    gesture.current = {
      pointerId: e.pointerId,
      key: rowKey(entry),
      enabled: entry.enabled,
      startY: e.clientY,
      rowHeight: e.currentTarget.offsetHeight || 1,
      moved: false,
    };
  };

  const onRowPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const g = gesture.current;
    if (!g || g.pointerId !== e.pointerId) return;
    if (!g.moved && Math.abs(e.clientY - g.startY) < 3) return;
    g.moved = true;
    const list = listRef.current;
    if (!list) return;
    const rect = list.getBoundingClientRect();
    const y = e.clientY - rect.top + list.scrollTop;
    // Always recompute from the COMMITTED base — no incremental
    // drift as the overlay reshuffles under the pointer.
    const next = movedOrder(entries, g.key, g.enabled, y, g.rowHeight);
    setOverlay(next);
  };

  const onRowPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    const g = gesture.current;
    if (!g || g.pointerId !== e.pointerId) return;
    gesture.current = null;
    if (!g.moved || overlay === null) {
      setOverlay(null);
      return;
    }
    // Commit; keep the overlay showing until the inventory event
    // round-trips the truth. A rejection (stale base) re-fetches.
    const order = overlay.map((entry) => ({
      identityKey: entry.identityKey,
      name: entry.name,
    }));
    void tabsReorder(order).catch(async () => {
      const inventory = await tabsInventory();
      if (inventory) setEntries(inventory);
      setOverlay(null);
    });
  };

  return (
    <div
      ref={listRef}
      className={cn("flex-1", "min-h-0", "overflow-y-auto")}
    >
      {display.length === 0 && (
        <div
          className={cn(
            "p-6",
            "font-mono",
            "text-[11px]",
            "text-info-dim",
            "select-none",
          )}
        >
          no tabs loaded
        </div>
      )}
      {display.map((entry) => (
        <TabRow
          key={rowKey(entry)}
          entry={entry}
          dragging={
            gesture.current?.moved === true &&
            gesture.current.key === rowKey(entry)
          }
          onPointerDown={onRowPointerDown}
          onPointerMove={onRowPointerMove}
          onPointerUp={onRowPointerUp}
          onPointerCancel={cancelGesture}
          onLostPointerCapture={cancelGesture}
        />
      ))}
    </div>
  );
}

function rowKey(entry: { identityKey: string; name: string }): string {
  return `${entry.identityKey}\n${entry.name}`;
}

/** The new display order for dragging `key` (an `enabled` row drags
 * its BLOCK; a disabled row drags itself) to pointer offset `y` —
 * or `null` when nothing changes. Pure function of the committed
 * base. */
function movedOrder(
  base: TabInventoryEntry[],
  key: string,
  enabled: boolean,
  y: number,
  rowHeight: number,
): TabInventoryEntry[] | null {
  const from = base.findIndex((entry) => rowKey(entry) === key);
  if (from < 0) return null;
  // The dragged block: an enabled row + its maximal trailing run of
  // MIDDLE disabled rows (a trailing run with no enabled row after
  // it is end-pinned and never rides).
  let blockEnd = from;
  if (enabled) {
    let j = from + 1;
    while (j < base.length && !base[j].enabled) j++;
    if (j < base.length) blockEnd = j - 1;
  }
  const blockLen = blockEnd - from + 1;

  // Raw insertion boundary by nearest row edge.
  const raw = Math.max(0, Math.min(base.length, Math.round(y / rowHeight)));

  // Allowed boundaries. Disabled rows: anywhere (re-parenting).
  // Enabled blocks: block boundaries only, after the start-pinned
  // disabled run and not past the start of the end-pinned run.
  let boundary = raw;
  if (enabled) {
    let startPin = 0;
    while (startPin < base.length && !base[startPin].enabled) startPin++;
    let endPin = base.length;
    while (endPin > 0 && !base[endPin - 1].enabled) endPin--;
    const allowed: number[] = [];
    for (let p = startPin; p <= endPin; p++) {
      if (p === startPin || p === endPin || base[p].enabled) allowed.push(p);
    }
    boundary = allowed.reduce(
      (best, p) => (Math.abs(p - raw) < Math.abs(best - raw) ? p : best),
      allowed[0] ?? raw,
    );
  }

  // Remove the block, re-aim the boundary, splice it back in.
  const without = [...base];
  const block = without.splice(from, blockLen);
  let target = boundary;
  if (target > from) target -= blockLen;
  target = Math.max(0, Math.min(without.length, target));
  without.splice(target, 0, ...block);

  const changed = without.some((entry, i) => entry !== base[i]);
  return changed ? without : null;
}

/** One inventory row: icon, identity, tab title, toggle. Permanent
 * rows are greyed and inert (not draggable, not toggleable) but can
 * be displaced by other rows' moves. */
function TabRow({
  entry,
  dragging,
  onPointerDown,
  onPointerMove,
  onPointerUp,
  onPointerCancel,
  onLostPointerCapture,
}: {
  entry: TabInventoryEntry;
  dragging: boolean;
  onPointerDown: (
    e: React.PointerEvent<HTMLDivElement>,
    entry: TabInventoryEntry,
  ) => void;
  onPointerMove: (e: React.PointerEvent<HTMLDivElement>) => void;
  onPointerUp: (e: React.PointerEvent<HTMLDivElement>) => void;
  onPointerCancel: () => void;
  onLostPointerCapture: () => void;
}) {
  const iconUrl = tabIconUrl(entry.identity, entry.icon);
  return (
    <div
      data-inventory-row
      onPointerDown={(e) => onPointerDown(e, entry)}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerCancel}
      onLostPointerCapture={onLostPointerCapture}
      className={cn(
        "flex",
        "items-center",
        "gap-3",
        "px-4",
        "py-2",
        "border-b",
        "border-node-border",
        "font-mono",
        "text-sm",
        "select-none",
        "touch-none",
        entry.permanent ? "opacity-40" : "cursor-grab",
        dragging && "opacity-60",
      )}
    >
      {iconUrl !== undefined ? (
        <IdentityIcon
          url={iconUrl}
          className={cn("w-3.5", "h-3.5", "shrink-0")}
        />
      ) : (
        <span className={cn("w-3.5", "shrink-0")} />
      )}
      <span
        data-inventory-identity
        className={cn(
          "shrink-0",
          "text-xs",
          "text-info-dim",
          "truncate",
          "max-w-64",
        )}
      >
        {entry.identity}
      </span>
      <span
        data-inventory-title
        className={cn("min-w-0", "truncate", "text-info-bright")}
      >
        {entry.title}
      </span>
      <Toggle entry={entry} />
    </div>
  );
}

/** The enabled toggle — a pill switch. Disabled (greyed, inert) for
 * permanent entries; state comes from the inventory (the emit after
 * each toggle round-trips the truth — no optimistic local state). */
function Toggle({ entry }: { entry: TabInventoryEntry }) {
  return (
    <button
      type="button"
      data-inventory-toggle
      disabled={entry.permanent}
      aria-checked={entry.enabled}
      role="switch"
      title={
        entry.permanent
          ? "permanent"
          : entry.enabled
            ? "disable this tab"
            : "enable this tab"
      }
      // The row owns the drag — the pill must not start one (the
      // strip's ✕-button precedent).
      onPointerDown={(e) => e.stopPropagation()}
      onClick={() => {
        if (!entry.permanent) {
          tabsToggle(entry.identityKey, entry.name, !entry.enabled);
        }
      }}
      className={cn(
        "ml-auto",
        "shrink-0",
        "relative",
        "w-8",
        "h-4",
        "rounded-full",
        "transition-colors",
        entry.enabled ? "bg-copper-mid" : "bg-ground-surface",
        entry.permanent ? "cursor-not-allowed" : "cursor-pointer",
        "border",
        "border-node-border",
      )}
    >
      <span
        className={cn(
          "absolute",
          "top-[1px]",
          "w-3",
          "h-3",
          "rounded-full",
          "transition-[left]",
          entry.enabled
            ? cn("left-[17px]", "bg-info-full")
            : cn("left-[1px]", "bg-info-dim"),
        )}
      />
    </button>
  );
}

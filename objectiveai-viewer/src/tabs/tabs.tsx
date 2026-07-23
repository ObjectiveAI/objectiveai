import { useEffect, useState } from "react";
import cn from "classnames";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import { restrictToVerticalAxis } from "@dnd-kit/modifiers";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
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
 * STICKINESS (applied at DROP time; dnd-kit owns the sensors and
 * shift animations): dragging an ENABLED row carries its trailing
 * run of MIDDLE disabled rows (sandwiched runs belong to the
 * enabled row above them); the drop snaps to block boundaries, and
 * never before the leading disabled run nor past the trailing one —
 * start/end disabled rows stay pinned. Dragging a DISABLED row
 * moves just itself, anywhere (re-parenting is the point).
 *
 * Pure view over the Rust inventory — subscribe FIRST
 * (`inventory://changed` carries the full ordered list on every
 * change), then pull. A drop shows its computed order immediately
 * (local overlay); the commit round-trips through Rust and the next
 * inventory event is truth. */
export default function TabsTab() {
  const [entries, setEntries] = useState<TabInventoryEntry[]>([]);
  const [overlay, setOverlay] = useState<TabInventoryEntry[] | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void (async () => {
      unlisten = await tauriListen<TabInventoryEntry[]>(
        "inventory://changed",
        (e) => {
          if (disposed) return;
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

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 3 } }),
  );

  const onDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    const dragged = display.find((entry) => rowKey(entry) === active.id);
    const to = display.findIndex((entry) => rowKey(entry) === over.id);
    if (!dragged || to < 0) return;
    const next = movedOrder(display, rowKey(dragged), dragged.enabled, to);
    if (next === null) return;
    setOverlay(next);
    const order = next.map((entry) => ({
      identityKey: entry.identityKey,
      name: entry.name,
    }));
    void tabsReorder(order).catch(async () => {
      // Rejected (the base raced a change) — re-fetch truth.
      const inventory = await tabsInventory();
      if (inventory) setEntries(inventory);
      setOverlay(null);
    });
  };

  return (
    // overflow-x-hidden: dnd transforms can momentarily poke past
    // the right edge — never show a horizontal scrollbar for it.
    <div className={cn("flex-1", "min-h-0", "overflow-y-auto", "overflow-x-hidden")}>
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
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        // Vertical only — a row can never poke sideways (no width
        // scroller; overflow-x-hidden stays as belt).
        modifiers={[restrictToVerticalAxis]}
        onDragEnd={onDragEnd}
      >
        <SortableContext
          items={display.map(rowKey)}
          strategy={verticalListSortingStrategy}
        >
          {display.map((entry) => (
            <TabRow key={rowKey(entry)} entry={entry} />
          ))}
        </SortableContext>
      </DndContext>
    </div>
  );
}

function rowKey(entry: { identityKey: string; name: string }): string {
  return `${entry.identityKey}\n${entry.name}`;
}

/** The new display order for dropping `key` (an `enabled` row drags
 * its BLOCK; a disabled row drags itself) onto row index `to` — or
 * `null` when nothing changes. */
function movedOrder(
  base: TabInventoryEntry[],
  key: string,
  enabled: boolean,
  to: number,
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

  // Sortable semantics: dropping ON index `to` inserts before it
  // when moving up, after it when moving down.
  const raw = to > from ? to + 1 : to;

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
    if (allowed.length === 0) return null;
    boundary = allowed.reduce(
      (best, p) => (Math.abs(p - raw) < Math.abs(best - raw) ? p : best),
      allowed[0],
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
function TabRow({ entry }: { entry: TabInventoryEntry }) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: rowKey(entry), disabled: entry.permanent });
  const iconUrl = tabIconUrl(entry.identity, entry.icon);
  return (
    <div
      ref={setNodeRef}
      data-inventory-row
      style={{
        transform: transform
          ? `translate3d(${transform.x}px, ${transform.y}px, 0)`
          : undefined,
        transition,
      }}
      {...attributes}
      {...listeners}
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
        "bg-ground",
        entry.permanent ? "opacity-40" : "cursor-grab",
        isDragging && cn("opacity-60", "relative", "z-10"),
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
      // drag sensor activates on pointerdown).
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

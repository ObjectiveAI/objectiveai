import { useEffect, useRef, useState, type ReactElement } from "react";
import cn from "classnames";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { restrictToHorizontalAxis } from "@dnd-kit/modifiers";
import {
  SortableContext,
  horizontalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { IdentityIcon } from "./shared/IdentityIcon";
import {
  tabIconUrl,
  tabsClose,
  tabsDetach,
  tabsMove,
  tabsSelect,
  type TabDesc,
} from "../lib/tabs";

/**
 * The unified tab strip — one per window, always the window's top
 * band (exactly 40 logical px: the Rust docking hit-test bakes that
 * height in as `STRIP_HEIGHT_LOGICAL`).
 *
 * - Horizontal scroll on overflow: hidden scrollbar, mouse wheel
 *   scrubs (`deltaY → scrollLeft`), the active tab keeps itself in
 *   view.
 * - Click selects; ✕ closes (closable kinds only — home tabs are
 *   permanent).
 * - Drag WITHIN the strip reorders — dnd-kit sortable, locked to the
 *   HORIZONTAL axis (rows animate aside; the visual can't leave the
 *   band), committed on drop via `tabs_move` (no optimistic reorder —
 *   the model snapshot is the truth and would revert it anyway).
 * - Dragging past the strip (≈40px of RAW vertical pointer travel —
 *   the axis lock is visual only, the deltas still flow — or out of
 *   the window) DETACHES: the DndContext is REMOUNTED (killing the
 *   in-flight drag; the OS window drag steals input from this
 *   webview and pointer events may never arrive again) before
 *   `tabs_detach` fires.
 * - `dockPreview` (the Rust docking task's hover signal) highlights
 *   the strip as a drop target.
 */
export function TabStrip({
  tabs,
  activeId,
  dockPreview,
}: {
  tabs: TabDesc[];
  activeId: number;
  dockPreview: boolean;
}): ReactElement {
  const stripRef = useRef<HTMLElement | null>(null);
  // Bumping remounts the DndContext — the hard reset after a detach
  // hands this gesture to the OS.
  const [dndEpoch, setDndEpoch] = useState(0);
  const gesture = useRef<{
    tabId: number;
    startY: number;
    detached: boolean;
    cleanup: () => void;
  } | null>(null);
  // A completed drag fires a trailing click on the dragged tab —
  // swallow exactly one.
  const suppressClick = useRef(false);

  // Unmount safety: never leak a gesture's raw-pointer listener.
  useEffect(() => {
    return () => {
      gesture.current?.cleanup();
      gesture.current = null;
    };
  }, []);

  // Keep the active tab visible whenever it changes. HORIZONTAL
  // ONLY, by hand — scrollIntoView's block axis nudged the nav's
  // hair of vertical scrollability and set off a jitter loop.
  useEffect(() => {
    const strip = stripRef.current;
    if (!strip) return;
    const el = strip.querySelector<HTMLElement>(`[data-tab-id="${activeId}"]`);
    if (!el) return;
    const left = el.offsetLeft - strip.scrollLeft;
    const right = left + el.offsetWidth;
    if (left < 0) {
      strip.scrollLeft += left;
    } else if (right > strip.clientWidth) {
      strip.scrollLeft += right - strip.clientWidth;
    }
  }, [activeId]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 3 } }),
  );

  const onDragStart = (event: DragStartEvent) => {
    if (typeof event.active.id !== "number") return;
    const activator = event.activatorEvent as PointerEvent;
    // Detach detection rides a RAW window pointermove listener for
    // this gesture only: the horizontal-axis modifier zeroes dnd's
    // own event deltas (delta.y is ALWAYS 0 under it), so vertical
    // travel is invisible to onDragMove — the OS coordinates aren't.
    const onRawMove = (e: PointerEvent) => {
      const g = gesture.current;
      if (!g || g.detached) return;
      const outOfBand =
        Math.abs(e.clientY - g.startY) > 40 ||
        e.clientX < 0 ||
        e.clientY < 0 ||
        e.clientX > window.innerWidth ||
        e.clientY > window.innerHeight;
      if (!outOfBand) return;
      // Detach ONCE: kill the dnd gesture FIRST (remount — the OS
      // drag takes over and this webview may never see another
      // pointer event), then hand the tab to Rust.
      g.detached = true;
      g.cleanup();
      setDndEpoch((epoch) => epoch + 1);
      tabsDetach(g.tabId);
    };
    window.addEventListener("pointermove", onRawMove);
    gesture.current = {
      tabId: event.active.id,
      startY: activator.clientY ?? 0,
      detached: false,
      cleanup: () => window.removeEventListener("pointermove", onRawMove),
    };
  };

  const onDragEnd = (event: DragEndEvent) => {
    const g = gesture.current;
    g?.cleanup();
    gesture.current = null;
    suppressClick.current = true;
    if (!g || g.detached) return;
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    const from = tabs.findIndex((t) => t.id === active.id);
    const to = tabs.findIndex((t) => t.id === over.id);
    if (from < 0 || to < 0 || from === to) return;
    if (typeof active.id === "number") {
      tabsMove(active.id, to);
    }
  };

  const onTabClick = (tabId: number) => {
    if (suppressClick.current) {
      suppressClick.current = false;
      return;
    }
    tabsSelect(tabId);
  };

  return (
    <nav
      ref={stripRef}
      role="tablist"
      onWheel={(e) => {
        const strip = stripRef.current;
        if (strip && e.deltaY !== 0) {
          strip.scrollLeft += e.deltaY;
        }
      }}
      className={cn(
        "flex",
        "items-center",
        "gap-1",
        "px-2",
        "h-10",
        "shrink-0",
        "border-b",
        "bg-ground-raised",
        "overflow-x-auto",
        "overflow-y-hidden",
        "[scrollbar-width:none]",
        "[&::-webkit-scrollbar]:hidden",
        dockPreview ? "border-copper-hot" : "border-node-border",
        dockPreview && "bg-copper-warm/10",
      )}
    >
      <DndContext
        key={dndEpoch}
        sensors={sensors}
        collisionDetection={closestCenter}
        // Horizontal only — tabs slide along the band; the raw
        // pointer deltas (used for detach detection) are unaffected.
        modifiers={[restrictToHorizontalAxis]}
        onDragStart={onDragStart}
        onDragEnd={onDragEnd}
        onDragCancel={() => {
          gesture.current?.cleanup();
          gesture.current = null;
        }}
      >
        <SortableContext
          items={tabs.map((t) => t.id)}
          strategy={horizontalListSortingStrategy}
        >
          {tabs.map((tab) => (
            <TabItem
              key={tab.id}
              tab={tab}
              active={tab.id === activeId}
              onClick={onTabClick}
            />
          ))}
        </SortableContext>
      </DndContext>
    </nav>
  );
}

/** One strip tab — a horizontal sortable item. */
function TabItem({
  tab,
  active,
  onClick,
}: {
  tab: TabDesc;
  active: boolean;
  onClick: (tabId: number) => void;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: tab.id });
  return (
    <div
      ref={setNodeRef}
      // Spreads FIRST — our role/aria then override dnd-kit's
      // attribute defaults (it ships role="button").
      {...attributes}
      {...listeners}
      data-tab-id={tab.id}
      role="tab"
      aria-selected={active}
      title={`${tab.kind.identity} - ${tab.title}`}
      style={{
        transform: transform
          ? `translate3d(${transform.x}px, 0, 0)`
          : undefined,
        transition,
      }}
      onClick={() => onClick(tab.id)}
      className={cn(
        "flex",
        "items-center",
        "gap-1.5",
        "px-3",
        // Total tab height must FIT the 40px band: 14px icon line +
        // 2px gap + 14px name line + 6px padding + 2px border = 38.
        // Anything taller makes the nav vertically scrollable by a
        // hair — the source of the strip jitter.
        "py-[3px]",
        "rounded-sm",
        "font-mono",
        "text-sm",
        "border-b-2",
        "transition-colors",
        "cursor-pointer",
        "select-none",
        "touch-none",
        "whitespace-nowrap",
        "max-w-56",
        "shrink-0",
        active
          ? cn(
              "border-copper-mid",
              "text-copper-bright",
              "font-semibold",
              "bg-ground-surface",
            )
          : cn("border-transparent", "text-info-dim", "hover:text-info-mid"),
        isDragging && cn("opacity-60", "relative", "z-10"),
      )}
    >
      {/* Identity over name, stacked — whose surface this is, then
          which one; the identity's icon (optional) sits to its
          left. */}
      <span className={cn("flex", "flex-col", "gap-0.5", "min-w-0")}>
        <span
          data-tab-identity
          className={cn(
            "flex",
            "items-center",
            "gap-1",
            "min-w-0",
            "text-xs",
            "leading-none",
            "text-info-dim",
          )}
        >
          {tabIconUrl(tab.kind.identity, tab.icon) !== undefined && (
            // Inlined when SVG: explicit fills render as authored;
            // currentColor inherits this line's color — the icon
            // chooses.
            <IdentityIcon
              url={tabIconUrl(tab.kind.identity, tab.icon)!}
              className={cn("w-3.5", "h-3.5", "shrink-0")}
            />
          )}
          <span className={cn("truncate", "min-w-0")}>
            {tab.kind.identity}
          </span>
        </span>
        <span className={cn("truncate", "min-w-0", "leading-none")}>
          {tab.title}
        </span>
      </span>
      {tab.closable && (
        <button
          type="button"
          aria-label={`close ${tab.title}`}
          onPointerDown={(e) => e.stopPropagation()}
          onClick={(e) => {
            e.stopPropagation();
            tabsClose(tab.id);
          }}
          className={cn(
            "rounded-sm",
            "px-0.5",
            "leading-none",
            "text-info-dim",
            "hover:text-copper-hot",
            "cursor-pointer",
          )}
        >
          ×
        </button>
      )}
    </div>
  );
}

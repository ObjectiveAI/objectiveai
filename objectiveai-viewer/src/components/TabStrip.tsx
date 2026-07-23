import { useEffect, useRef, useState, type ReactElement } from "react";
import cn from "classnames";
import {
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
 * - Click selects; ✕ closes (closable kinds only — the two home tabs
 *   are permanent).
 * - Pointer drag WITHIN the strip reorders (committed on release via
 *   `tabs_move`; no optimistic reorder — the registry snapshot is the
 *   truth and would revert it anyway).
 * - Dragging past the strip (≈40px vertically or out of the window)
 *   DETACHES: local drag state is reset and pointer capture released
 *   BEFORE invoking `tabs_detach` — the OS window drag steals input
 *   from this webview and `pointercancel` delivery is not guaranteed.
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
  const drag = useRef<{
    tabId: number;
    pointerId: number;
    startX: number;
    startY: number;
    moved: boolean;
    detached: boolean;
  } | null>(null);
  // Index the dragged tab would land at, for the insertion hint.
  const [dropIndex, setDropIndex] = useState<number | null>(null);
  const [draggingId, setDraggingId] = useState<number | null>(null);

  // Keep the active tab visible whenever it changes.
  useEffect(() => {
    const strip = stripRef.current;
    if (!strip) return;
    const el = strip.querySelector(`[data-tab-id="${activeId}"]`);
    el?.scrollIntoView({ inline: "nearest", block: "nearest" });
  }, [activeId]);

  /** The strip index the pointer's x maps to (by tab midpoints). */
  const indexAt = (clientX: number): number => {
    const strip = stripRef.current;
    if (!strip) return 0;
    const els = Array.from(
      strip.querySelectorAll<HTMLElement>("[data-tab-id]"),
    );
    let index = els.length;
    for (let i = 0; i < els.length; i += 1) {
      const rect = els[i].getBoundingClientRect();
      if (clientX < rect.left + rect.width / 2) {
        index = i;
        break;
      }
    }
    return index;
  };

  const resetDrag = () => {
    drag.current = null;
    setDropIndex(null);
    setDraggingId(null);
  };

  const onPointerDown = (e: React.PointerEvent, tab: TabDesc) => {
    // Mouse-primary only; the ✕ button stops propagation itself.
    if (e.button !== 0) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag.current = {
      tabId: tab.id,
      pointerId: e.pointerId,
      startX: e.clientX,
      startY: e.clientY,
      moved: false,
      detached: false,
    };
  };

  const onPointerMove = (e: React.PointerEvent) => {
    const d = drag.current;
    if (!d || d.detached || e.pointerId !== d.pointerId) return;
    const dx = e.clientX - d.startX;
    const dy = e.clientY - d.startY;
    if (!d.moved && Math.abs(dx) < 3 && Math.abs(dy) < 3) return;
    d.moved = true;
    setDraggingId(d.tabId);
    const outOfBand =
      Math.abs(dy) > 40 ||
      e.clientX < 0 ||
      e.clientY < 0 ||
      e.clientX > window.innerWidth ||
      e.clientY > window.innerHeight;
    if (outOfBand) {
      // Detach ONCE: release + reset FIRST — the OS drag takes over
      // and this webview may never see another pointer event.
      d.detached = true;
      const tabId = d.tabId;
      try {
        (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
      } catch {
        // Capture already gone — fine.
      }
      resetDrag();
      tabsDetach(tabId);
      return;
    }
    setDropIndex(indexAt(e.clientX));
  };

  const onPointerUp = (e: React.PointerEvent, tab: TabDesc) => {
    const d = drag.current;
    if (!d || e.pointerId !== d.pointerId) return;
    const wasMoved = d.moved;
    const target = dropIndex;
    resetDrag();
    if (!wasMoved) {
      tabsSelect(tab.id);
      return;
    }
    if (target !== null) {
      // Dropping right of the removed slot shifts left by one.
      const from = tabs.findIndex((t) => t.id === d.tabId);
      const index = from >= 0 && target > from ? target - 1 : target;
      if (from !== index) {
        tabsMove(d.tabId, index);
      }
    }
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
        "[scrollbar-width:none]",
        "[&::-webkit-scrollbar]:hidden",
        dockPreview ? "border-copper-hot" : "border-node-border",
        dockPreview && "bg-copper-warm/10",
      )}
    >
      {tabs.map((tab, i) => {
        const active = tab.id === activeId;
        return (
          <div
            key={tab.id}
            data-tab-id={tab.id}
            role="tab"
            aria-selected={active}
            title={`${tab.kind.identity} - ${tab.title}`}
            onPointerDown={(e) => onPointerDown(e, tab)}
            onPointerMove={onPointerMove}
            onPointerUp={(e) => onPointerUp(e, tab)}
            onPointerCancel={resetDrag}
            onLostPointerCapture={() => {
              if (drag.current && !drag.current.detached) resetDrag();
            }}
            className={cn(
              "flex",
              "items-center",
              "gap-1.5",
              "px-3",
              "py-1",
              "rounded-sm",
              "font-mono",
              "text-xs",
              "border-b-2",
              "transition-colors",
              "cursor-pointer",
              "select-none",
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
                : cn(
                    "border-transparent",
                    "text-info-dim",
                    "hover:text-info-mid",
                  ),
              draggingId === tab.id && "opacity-60",
              dropIndex === i &&
                draggingId !== null &&
                "border-l-2 border-l-copper-hot",
            )}
          >
            {/* Identity over name, stacked — whose surface this is,
                then which one. */}
            <span className={cn("flex", "flex-col", "gap-0.5", "min-w-0")}>
              <span
                data-tab-identity
                className={cn(
                  "truncate",
                  "text-xs",
                  "leading-none",
                  "text-info-dim",
                )}
              >
                {tab.kind.identity}
              </span>
              <span className={cn("truncate", "min-w-0", "leading-tight")}>
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
      })}
    </nav>
  );
}

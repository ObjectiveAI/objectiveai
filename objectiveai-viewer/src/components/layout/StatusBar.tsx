import cn from "classnames";
import type { Entry } from "../../types";
import { formatCost } from "../../lib/format";
import {
  toggleOrientation,
  useOrientation,
} from "../../hooks/useOrientation";

export function StatusBar({
  entries,
  activeAgents,
  zoom,
  onZoomChange,
  isHistorical,
}: {
  entries: Entry[];
  /** Count of currently-active agents — from the app's agents-list
   * connection, threaded down by App (no global subscriptions here). */
  activeAgents: number;
  /** Canvas zoom factor (1 = 100%), with its setter — the slider
   * lives here so it spans every tab; only the main canvas consumes
   * it for now. */
  zoom: number;
  onZoomChange: (zoom: number) => void;
  isHistorical?: boolean;
}) {
  let totalTokens = 0;
  let totalCost = 0;
  for (const e of entries) {
    const usage = e.chunk && "usage" in e.chunk ? (e.chunk as { usage?: { total_tokens?: number; cost?: number } }).usage : undefined;
    if (usage) {
      totalTokens += usage.total_tokens ?? 0;
      totalCost += typeof usage.cost === "number" ? usage.cost : 0;
    }
  }

  return (
    // Fixed h-8 (32px): the Rust side carves the content webviews'
    // rect out of the window as strip (40) + footer (32) — keep in
    // sync with STATUS_HEIGHT_LOGICAL (shell/native.rs).
    <footer role="status" aria-live="polite" className={cn("flex", "items-center", "h-8", "shrink-0", "gap-4", "px-4", "border-t", "border-node-border", "bg-ground-raised", "font-mono", "text-[10px]", "text-info-dim", "tabular-nums", "select-none", "overflow-hidden", "whitespace-nowrap", "min-w-0")}>
      <div className={cn("flex", "items-center", "gap-1.5", "shrink-0")}>
        <div className={cn("w-1.5", "h-1.5", "rounded-full", activeAgents > 0 ? cn("bg-copper-hot", "animate-pulse") : "bg-info-dim")} />
        <span>{activeAgents} active {activeAgents === 1 ? "agent" : "agents"}</span>
      </div>
      {isHistorical && (
        <div className={cn("flex", "items-center", "gap-1.5", "shrink-0")}>
          <div className={cn("w-1.5", "h-1.5", "rounded-full", "bg-info-dim")} />
          <span>Historical</span>
        </div>
      )}
      {totalTokens > 0 && <span className={cn("shrink-0")}>{totalTokens.toLocaleString()} tokens</span>}
      {totalCost > 0 && <span className={cn("shrink-0")}>{formatCost(totalCost)}</span>}
      {/* Canvas view controls — pinned to the footer's right edge. */}
      <div className={cn("ml-auto", "flex", "items-center", "gap-3", "shrink-0")}>
        <OrientationToggle />
        <input
          type="range"
          data-zoom-slider
          min={0.25}
          max={2}
          step={0.05}
          value={zoom}
          onChange={(e) => onZoomChange(Number(e.target.value))}
          aria-label="Canvas zoom"
          className={cn("w-24", "h-1", "cursor-pointer", "accent-[#d97706]")}
        />
        <button
          type="button"
          data-zoom-reset
          onClick={() => onZoomChange(1)}
          title="Reset zoom"
          className={cn(
            "tabular-nums",
            "w-9",
            "text-right",
            "hover:text-info-bright",
            "cursor-pointer",
          )}
        >
          {Math.round(zoom * 100)}%
        </button>
      </div>
    </footer>
  );
}

/** The hierarchy-orientation toggle: shows the CURRENT descent
 * direction ("↓ deep" = tiers top-down, "→ wide" = tiers
 * left-to-right); clicking flips it. State lives in the
 * [`useOrientation`] module store so the tree consumes it as a hook
 * with no prop threading. */
function OrientationToggle() {
  const orientation = useOrientation();
  return (
    <button
      type="button"
      data-orientation-toggle
      onClick={toggleOrientation}
      title="Toggle hierarchy orientation"
      className={cn("hover:text-info-bright", "cursor-pointer")}
    >
      {orientation === "vertical" ? "↓ deep" : "→ wide"}
    </button>
  );
}

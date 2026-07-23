import { useEffect, useState } from "react";
import cn from "classnames";
import { tauriListen } from "../lib/tauri";
import {
  tabIconUrl,
  tabsInventory,
  tabsToggle,
  type TabInventoryEntry,
} from "../lib/tabs";
import { IdentityIcon } from "../components/shared/IdentityIcon";

/** The tabs home tab: every tab loaded from the system THIS boot —
 * the root objectiveai tabs and every scanned plugin tab — each with
 * its identity icon and a persisted enabled/disabled toggle. Toggle
 * off closes the live tab; on reopens it (into this window). The
 * tabs tab itself is listed greyed-out: permanent, not toggleable.
 * Pure view over the Rust inventory — subscribe FIRST
 * (`inventory://changed` carries the full list on every toggle),
 * then pull. */
export default function TabsTab() {
  const [entries, setEntries] = useState<TabInventoryEntry[]>([]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void (async () => {
      unlisten = await tauriListen<TabInventoryEntry[]>(
        "inventory://changed",
        (e) => {
          if (!disposed) setEntries(e.payload);
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

  return (
    <div className={cn("flex-1", "min-h-0", "overflow-y-auto")}>
      {entries.length === 0 && (
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
      {entries.map((entry) => (
        <TabRow key={`${entry.identityKey}\n${entry.name}`} entry={entry} />
      ))}
    </div>
  );
}

/** One inventory row: icon, identity, tab title, toggle. Permanent
 * rows are greyed and inert. */
function TabRow({ entry }: { entry: TabInventoryEntry }) {
  const iconUrl = tabIconUrl(entry.identity, entry.icon);
  return (
    <div
      data-inventory-row
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
        entry.permanent && "opacity-40",
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
        className={cn("shrink-0", "text-xs", "text-info-dim", "truncate", "max-w-64")}
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

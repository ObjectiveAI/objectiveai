import { useState, useMemo, useEffect, useDeferredValue, type ReactNode } from "react";
import cn from "classnames";
import * as Tooltip from "@radix-ui/react-tooltip";
import { tauriInvoke } from "./lib/tauri";
import { startDaemonListener } from "./daemon-listener";
import { registerActiveAgentsHandler } from "./hooks/useActiveAgents";
import { registerAgentCompletionsHandler } from "./listener-handlers/agentCompletions";
import { useEntries } from "./hooks/useEntries";
import { useCollapseState } from "./hooks/useCollapseState";
import { useSessionStorage } from "./hooks/useSessionStorage";
import { Shell } from "./components/layout/Shell";
import { StatusBar } from "./components/layout/StatusBar";
import { EntryView } from "./components/views/EntryView";
import { TabBar, type Tab } from "./TabBar";
import { PluginPane } from "./PluginPane";
import { RestoreBanner } from "./components/shared/RestoreBanner";
import { SessionPicker } from "./components/shared/SessionPicker";
import { SessionSidebar } from "./components/shared/SessionSidebar";
import { CommandPalette } from "./components/shared/CommandPalette";
import { DetailPanel } from "./components/shared/DetailPanel";
import { LogoMark, Wordmark } from "./components/shared/Logo";
import type { Entry } from "./types";
import { isTauri } from "./lib/tauri";
import { mockEntries } from "./mockEntries";

function Tip({ label, children }: { label: string; children: ReactNode }) {
  return (
    <Tooltip.Root delayDuration={400}>
      <Tooltip.Trigger asChild>{children}</Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content className={cn("bg-ground-surface", "border", "border-node-border", "rounded-sm", "px-2", "py-1", "font-mono", "text-[10px]", "text-info-mid", "shadow-lg")} sideOffset={6}>
          {label}
          <Tooltip.Arrow className={cn("fill-ground-surface")} />
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}

const KINDS: { kind: Entry["kind"]; label: string; activeClass: string }[] = [
  { kind: "agent-completion", label: "Agent", activeClass: "bg-kind-agent/20 text-kind-agent" },
  { kind: "execution", label: "Execution", activeClass: "bg-kind-execution/20 text-kind-execution" },
];

function ObjectiveAIView({ onStatusChange }: { onStatusChange?: (status: ViewerStatus) => void }) {
  const realEntries = useEntries();
  const liveEntries = !isTauri() && realEntries.length === 0 ? mockEntries : realEntries;
  const session = useSessionStorage(liveEntries, true);
  const [sessionPickerOpen, setSessionPickerOpen] = useState(false);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);

  useEffect(() => {
    if (liveEntries.length > 0 && session.showRestoreBanner) {
      session.dismissRestore();
    }
  }, [liveEntries.length, session.showRestoreBanner, session.dismissRestore]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setCommandPaletteOpen((v) => !v);
        return;
      }
      if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "j" || e.key === "k") {
        const active = document.activeElement;
        if (active?.tagName === "INPUT" || active?.tagName === "TEXTAREA") return;
        const triggers = Array.from(document.querySelectorAll<HTMLElement>("[data-entry-trigger]"));
        if (triggers.length === 0) return;
        const currentIdx = active ? triggers.indexOf(active as HTMLElement) : -1;
        const direction = (e.key === "ArrowDown" || e.key === "j") ? 1 : -1;
        const nextIdx = currentIdx === -1 ? (direction === 1 ? 0 : triggers.length - 1) : Math.max(0, Math.min(triggers.length - 1, currentIdx + direction));
        triggers[nextIdx]?.focus();
        e.preventDefault();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const isRestoredNotBrowsing = session.restoredEntries && !session.isViewingPast;
  const entries = (isRestoredNotBrowsing && liveEntries.length > 0)
    ? liveEntries
    : (session.restoredEntries ?? liveEntries);
  const { isCollapsed, toggle, collapseAll, expandAll } = useCollapseState(entries);
  const [selectedEntryId, setSelectedEntryId] = useState<string | null>(!isTauri() ? "ac-complete-001" : null);
  const [activeKinds, setActiveKinds] = useState<Set<Entry["kind"]>>(
    new Set(KINDS.map((k) => k.kind))
  );
  const [searchQuery, setSearchQuery] = useState("");
  const deferredSearch = useDeferredValue(searchQuery);

  const toggleKind = (kind: Entry["kind"]) => {
    setActiveKinds((prev) => {
      const next = new Set(prev);
      if (next.has(kind)) {
        if (next.size > 1) next.delete(kind);
      } else {
        next.add(kind);
      }
      return next;
    });
  };

  // Report the status-bar inputs up to App — the bar spans every tab,
  // so it lives above the panes.
  useEffect(() => {
    onStatusChange?.({ entries, isHistorical: session.isViewingPast });
  }, [entries, session.isViewingPast, onStatusChange]);

  const kindCounts = useMemo(() => {
    const counts = new Map<Entry["kind"], number>();
    for (const e of entries) counts.set(e.kind, (counts.get(e.kind) ?? 0) + 1);
    return counts;
  }, [entries]);

  const filtered = useMemo(() => {
    let result = entries.filter((e) => activeKinds.has(e.kind));
    if (deferredSearch.trim()) {
      const q = deferredSearch.trim().toLowerCase();
      result = result.filter((e) => {
        const json = JSON.stringify(e).toLowerCase();
        return json.includes(q);
      });
    }
    return result;
  }, [entries, activeKinds, deferredSearch]);

  const banner = session.showRestoreBanner && session.restoredEntries ? (
    <RestoreBanner
      entryCount={session.restoredEntries.length}
      timestamp={session.restoredTimestamp}
      onDismiss={session.dismissRestore}
      onBrowse={() => { session.dismissRestore(); session.refreshSessions().then(() => setSessionPickerOpen(true)); }}
    />
  ) : session.isViewingPast ? (
    <div className={cn("flex", "items-center", "gap-3", "px-4", "py-2", "border-b", "border-copper-warm/30", "bg-copper-warm/5", "text-xs", "text-info-mid", "select-none")}>
      <span>Viewing historical session</span>
      <button
        onClick={session.returnToLive}
        className={cn("ml-auto", "px-2", "py-0.5", "rounded-sm", "text-copper-bright", "hover:bg-copper-warm/20", "transition-colors")}
      >
        Return to live
      </button>
    </div>
  ) : null;

  const selectedEntry = selectedEntryId ? entries.find((e) => e.id === selectedEntryId) ?? null : null;

  const detailPanel = selectedEntry ? (
    <DetailPanel entry={selectedEntry} onClose={() => setSelectedEntryId(null)} />
  ) : null;

  return (
    <Shell banner={banner} entryCount={entries.length} sidebar={<SessionSidebar sessions={session.pastSessions} currentSessionId={session.sessionId} onLoad={(id) => { session.loadSession(id); }} />} detailPanel={detailPanel}>
      <SessionPicker
        open={sessionPickerOpen}
        onOpenChange={setSessionPickerOpen}
        sessions={session.pastSessions}
        currentSessionId={session.sessionId}
        onLoad={session.loadSession}
        onDelete={session.deleteSession}
        onRename={session.renameSession}
        onExport={session.exportSession}
      />
      <CommandPalette open={commandPaletteOpen} onOpenChange={setCommandPaletteOpen} />
      {entries.length > 0 && (
        <div className={cn("sticky", "top-0", "z-10", "flex", "items-center", "gap-1.5", "pb-3", "pt-0", "mb-1", "bg-ground/95", "backdrop-blur-sm", "select-none")}>
          {KINDS.map(({ kind, label, activeClass }) => {
            const count = kindCounts.get(kind) ?? 0;
            const active = activeKinds.has(kind);
            return (
              <button
                key={kind}
                onClick={() => toggleKind(kind)}
                className={cn(
                  "px-2.5",
                  "py-1",
                  "rounded-sm",
                  "font-mono",
                  "text-[10px]",
                  "transition-colors",
                  active
                    ? activeClass
                    : cn("bg-ground-surface", "text-info-dim", "hover:text-info-mid"),
                )}
              >
                {label}
                {count > 0 && <span className={cn("ml-1", "opacity-70")}>({count})</span>}
              </button>
            );
          })}
          <div className={cn("ml-auto", "flex", "items-center", "gap-1")}>
            <div className={cn("group/search", "relative", "mr-1")}>
              <svg className={cn("absolute", "left-2", "top-1/2", "-translate-y-1/2", "w-3", "h-3", "text-info-dim", "group-focus-within/search:text-copper-dim", "pointer-events-none", "transition-colors")} viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                <circle cx="5" cy="5" r="3.5" />
                <path d="M8 8L10.5 10.5" />
              </svg>
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search…"
                aria-label="Search entries"
                className={cn("w-28", "pl-6", "pr-2", "py-1", "rounded-sm", "bg-ground-surface", "border", "border-node-border", "text-[10px]", "font-mono", "text-info-mid", "placeholder:text-info-dim/50", "outline-none", "focus:border-copper-dim", "focus:w-44", "transition-all")}
              />
            </div>
            <Tip label="Sessions">
              <button
                onClick={() => { session.refreshSessions().then(() => setSessionPickerOpen(true)); }}
                aria-label="Sessions"
                className={cn("p-1.5", "rounded-sm", "text-info-dim", "hover:text-info-bright", "hover:bg-ground-surface", "transition-colors")}
              >
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="6" cy="6" r="5" />
                  <path d="M6 3v3l2 1" />
                </svg>
              </button>
            </Tip>
            <Tip label="Collapse all">
              <button
                onClick={collapseAll}
                aria-label="Collapse all"
                className={cn("p-1.5", "rounded-sm", "text-info-dim", "hover:text-info-bright", "hover:bg-ground-surface", "transition-colors")}
              >
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M2 8L6 4L10 8" />
                  <path d="M2 11L6 7L10 11" />
                </svg>
              </button>
            </Tip>
            <Tip label="Expand all">
              <button
                onClick={expandAll}
                aria-label="Expand all"
                className={cn("p-1.5", "rounded-sm", "text-info-dim", "hover:text-info-bright", "hover:bg-ground-surface", "transition-colors")}
              >
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M2 4L6 8L10 4" />
                  <path d="M2 1L6 5L10 1" />
                </svg>
              </button>
            </Tip>
          </div>
        </div>
      )}

      {entries.length === 0 && (
        <div className={cn("flex", "flex-col", "items-center", "justify-center", "py-20", "px-6", "select-none")}>
          <LogoMark className={cn("h-8", "w-auto", "text-info-dim/30", "mb-2")} />
          <Wordmark className={cn("w-[140px]", "h-auto", "text-info-dim/30", "mb-8")} />
          <p className={cn("text-info-bright", "text-sm", "font-medium", "mb-1")}>No activity yet</p>
          <p className={cn("text-info-dim", "text-xs", "mb-8", "max-w-xs", "text-center")}>Run a command from the CLI or use the command palette to get started. Results stream here in real time.</p>
          <div className={cn("max-w-sm", "w-full", "space-y-2.5")}>
            <div className={cn("bg-ground-surface", "border", "border-node-border", "rounded-md", "px-4", "py-3")}>
              <div className={cn("text-[10px]", "font-mono", "text-info-dim", "uppercase", "tracking-wide", "mb-1.5")}>From the CLI</div>
              <code className={cn("text-[11px]", "font-mono", "text-copper-bright", "leading-relaxed", "block")}>objectiveai functions executions create</code>
              <p className={cn("text-[10px]", "text-info-dim", "mt-1")}>Events stream here automatically when the viewer is running.</p>
            </div>
            <div className={cn("bg-ground-surface", "border", "border-node-border", "rounded-md", "px-4", "py-3")}>
              <div className={cn("text-[10px]", "font-mono", "text-info-dim", "uppercase", "tracking-wide", "mb-1.5")}>From here</div>
              <p className={cn("text-[11px]", "text-info-mid")}>
                Press{' '}
                <kbd className={cn("px-1", "py-px", "rounded-sm", "bg-ground-raised", "border", "border-node-border", "text-info-bright", "font-mono", "text-[10px]")}>{navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}K</kbd>
                {' '}to open the command palette and run any ObjectiveAI command directly.
              </p>
            </div>
            <div className={cn("text-center", "text-[10px]", "text-info-dim/50", "mt-3", "font-mono")}>
              listening on localhost:5001
            </div>
          </div>
        </div>
      )}

      {filtered.map((entry) => (
        <EntryView key={entry.id} entry={entry} collapsed={isCollapsed(entry.id)} onToggle={() => { toggle(entry.id); setSelectedEntryId(isCollapsed(entry.id) ? entry.id : null); }} selected={entry.id === selectedEntryId} />
      ))}
    </Shell>
  );
}

const OBJECTIVEAI_TAB_ID = "objectiveai";

export interface ViewerPluginInfo {
  owner: string;
  name: string;
  version: string;
  iframe_src: string;
}

/** The status-bar inputs ObjectiveAIView reports up to App. */
interface ViewerStatus {
  entries: Entry[];
  isHistorical: boolean;
}

function App() {
  const [plugins, setPlugins] = useState<ViewerPluginInfo[]>([]);
  const [activeTab, setActiveTab] = useState<string>(OBJECTIVEAI_TAB_ID);
  const [status, setStatus] = useState<ViewerStatus>({
    entries: [],
    isHistorical: false,
  });

  // Viewer startup: register every execution handler FIRST (the
  // listener is live-only), then start the autonomous singleton —
  // which registers the built-in plugins/run forwarding itself. All
  // idempotent, so StrictMode's double effect is harmless.
  useEffect(() => {
    registerActiveAgentsHandler();
    registerAgentCompletionsHandler();
    startDaemonListener();
  }, []);

  useEffect(() => {
    tauriInvoke<ViewerPluginInfo[]>("list_plugins_with_viewer")
      .then((p) => { if (p) setPlugins(p); })
      .catch((e) => {
        // eslint-disable-next-line no-console
        console.warn("list_plugins_with_viewer failed:", e);
      });
  }, []);

  const tabs: Tab[] = [
    { id: OBJECTIVEAI_TAB_ID, label: "ObjectiveAI" },
    ...plugins.map((p) => ({ id: p.name, label: p.name })),
  ];

  if (plugins.length === 0) {
    return (
      <div className={cn("flex", "flex-col", "h-screen")}>
        <div className={cn("flex", "flex-col", "flex-1", "min-h-0")}>
          <ObjectiveAIView onStatusChange={setStatus} />
        </div>
        <StatusBar entries={status.entries} isHistorical={status.isHistorical} />
      </div>
    );
  }

  return (
    <div className={cn("flex", "flex-col", "h-screen")}>
      <div
        className={cn(
          "flex",
          "flex-row",
          "items-stretch",
          "bg-neutral-100",
          "dark:bg-neutral-900",
          "border-b",
          "border-neutral-300",
          "dark:border-neutral-700",
        )}
      >
        <div className={cn("flex-1", "min-w-0")}>
          <TabBar tabs={tabs} activeTab={activeTab} onSelect={setActiveTab} />
        </div>
      </div>
      <div
        className={cn(
          "relative",
          "flex",
          "flex-col",
          "flex-1",
          "min-h-0",
        )}
      >
        {/* Every pane stays mounted at all times; only the active one is
            shown (the rest are display:none). Keeping plugin iframes
            mounted means their JS keeps running and the bridge keeps
            their per-plugin Tauri subscription alive, so a plugin
            receives its routed daemon-stream events (`plugins_run`)
            regardless of which tab is focused. */}
        <div
          className={cn(
            "flex-col",
            "flex-1",
            "min-h-0",
            activeTab === OBJECTIVEAI_TAB_ID ? "flex" : "hidden",
          )}
        >
          <ObjectiveAIView onStatusChange={setStatus} />
        </div>
        {plugins.map((p) => (
          <div
            key={p.name}
            className={cn(
              "flex-col",
              "flex-1",
              "min-h-0",
              activeTab === p.name ? "flex" : "hidden",
            )}
          >
            <PluginPane info={p} />
          </div>
        ))}
      </div>
      {/* Spans every tab — plugin panes included. */}
      <StatusBar entries={status.entries} isHistorical={status.isHistorical} />
    </div>
  );
}

export default App;

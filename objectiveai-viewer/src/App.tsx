import { useState, useMemo, useEffect, useDeferredValue } from "react";
import { tauriInvoke } from "./lib/tauri";
import { useEntries } from "./hooks/useEntries";
import { useApiCalls } from "./hooks/useApiCalls";
import { useCollapseState } from "./hooks/useCollapseState";
import { useSessionStorage } from "./hooks/useSessionStorage";
import { Shell } from "./components/layout/Shell";
import { StatusBar } from "./components/layout/StatusBar";
import { EntryView } from "./components/views/EntryView";
import { NetworkPanel } from "./components/shared/NetworkPanel";
import { TabBar, type Tab } from "./TabBar";
import { PluginPane } from "./PluginPane";
import { RestoreBanner } from "./components/shared/RestoreBanner";
import { SessionPicker } from "./components/shared/SessionPicker";
import { SessionSidebar } from "./components/shared/SessionSidebar";
import { CommandPalette } from "./components/shared/CommandPalette";
import { LogoMark, Wordmark } from "./components/shared/Logo";
import type { Entry } from "./types";

const KINDS: { kind: Entry["kind"]; label: string; activeClass: string }[] = [
  { kind: "agent-completion", label: "Agent", activeClass: "bg-kind-agent/20 text-kind-agent" },
  { kind: "execution", label: "Execution", activeClass: "bg-kind-execution/20 text-kind-execution" },
  { kind: "invention", label: "Invention", activeClass: "bg-kind-invention/20 text-kind-invention" },
  { kind: "laboratory", label: "Laboratory", activeClass: "bg-kind-laboratory/20 text-kind-laboratory" },
];

function ObjectiveAIView() {
  const liveEntries = useEntries();
  const apiCalls = useApiCalls();
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
    <div className="flex items-center gap-3 px-4 py-2 border-b border-copper-warm/30 bg-copper-warm/5 text-xs text-info-mid select-none">
      <span>Viewing historical session</span>
      <button
        onClick={session.returnToLive}
        className="ml-auto px-2 py-0.5 rounded-sm text-copper-bright hover:bg-copper-warm/20 transition-colors"
      >
        Return to live
      </button>
    </div>
  ) : null;

  return (
    <Shell statusBar={<StatusBar entries={entries} isHistorical={session.isViewingPast} />} banner={banner} networkPanel={<NetworkPanel entries={apiCalls} />} entryCount={entries.length} sidebar={<SessionSidebar sessions={session.pastSessions} currentSessionId={session.sessionId} onLoad={(id) => { session.loadSession(id); }} />}>
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
        <div className="sticky top-0 z-10 flex items-center gap-1.5 pb-3 pt-0 mb-1 bg-ground/95 backdrop-blur-sm select-none">
          {KINDS.map(({ kind, label, activeClass }) => {
            const count = kindCounts.get(kind) ?? 0;
            const active = activeKinds.has(kind);
            return (
              <button
                key={kind}
                onClick={() => toggleKind(kind)}
                className={`px-2.5 py-1 rounded-sm font-mono text-[10px] transition-colors ${
                  active
                    ? activeClass
                    : "bg-ground-surface text-info-dim hover:text-info-mid"
                }`}
              >
                {label}
                {count > 0 && <span className="ml-1 opacity-70">({count})</span>}
              </button>
            );
          })}
          <div className="ml-auto flex items-center gap-1">
            <div className="relative mr-1">
              <svg className="absolute left-2 top-1/2 -translate-y-1/2 w-3 h-3 text-info-dim pointer-events-none" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                <circle cx="5" cy="5" r="3.5" />
                <path d="M8 8L10.5 10.5" />
              </svg>
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search…"
                aria-label="Search entries"
                className="w-28 pl-6 pr-2 py-1 rounded-sm bg-ground-surface border border-node-border text-[10px] font-mono text-info-mid placeholder:text-info-dim/50 outline-none focus:border-copper-dim focus:w-44 transition-all"
              />
            </div>
            <button
              onClick={() => { session.refreshSessions().then(() => setSessionPickerOpen(true)); }}
              title="Sessions"
              className="p-1.5 rounded-sm text-info-dim hover:text-info-bright hover:bg-ground-surface transition-colors"
            >
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="6" cy="6" r="5" />
                <path d="M6 3v3l2 1" />
              </svg>
            </button>
            <button
              onClick={collapseAll}
              title="Collapse all"
              className="p-1.5 rounded-sm text-info-dim hover:text-info-bright hover:bg-ground-surface transition-colors"
            >
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M2 8L6 4L10 8" />
                <path d="M2 11L6 7L10 11" />
              </svg>
            </button>
            <button
              onClick={expandAll}
              title="Expand all"
              className="p-1.5 rounded-sm text-info-dim hover:text-info-bright hover:bg-ground-surface transition-colors"
            >
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M2 4L6 8L10 4" />
                <path d="M2 1L6 5L10 1" />
              </svg>
            </button>
          </div>
        </div>
      )}

      {entries.length === 0 && (
        <div className="flex flex-col items-center justify-center py-20 px-6 select-none">
          <LogoMark className="h-8 w-auto text-info-dim/30 mb-2" />
          <Wordmark className="w-[140px] h-auto text-info-dim/30 mb-8" />
          <p className="text-info-bright text-sm font-medium mb-1">No activity yet</p>
          <p className="text-info-dim text-xs mb-8 max-w-xs text-center">Run a command from the CLI or use the command palette to get started. Results stream here in real time.</p>
          <div className="max-w-sm w-full space-y-2.5">
            <div className="bg-ground-surface border border-node-border rounded-md px-4 py-3">
              <div className="text-[10px] font-mono text-info-dim uppercase tracking-wide mb-1.5">From the CLI</div>
              <code className="text-[11px] font-mono text-copper-bright leading-relaxed block">objectiveai functions executions create</code>
              <p className="text-[10px] text-info-dim mt-1">Events stream here automatically when the viewer is running.</p>
            </div>
            <div className="bg-ground-surface border border-node-border rounded-md px-4 py-3">
              <div className="text-[10px] font-mono text-info-dim uppercase tracking-wide mb-1.5">From here</div>
              <p className="text-[11px] text-info-mid">
                Press{' '}
                <kbd className="px-1 py-px rounded-sm bg-ground-raised border border-node-border text-info-bright font-mono text-[10px]">{navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}K</kbd>
                {' '}to open the command palette and run any ObjectiveAI command directly.
              </p>
            </div>
            <div className="text-center text-[10px] text-info-dim/50 mt-3 font-mono">
              listening on localhost:5001
            </div>
          </div>
        </div>
      )}

      {filtered.map((entry) => (
        <EntryView key={entry.id} entry={entry} collapsed={isCollapsed(entry.id)} onToggle={() => toggle(entry.id)} />
      ))}
    </Shell>
  );
}

const OBJECTIVEAI_TAB_ID = "objectiveai";

function App() {
  const [pluginNames, setPluginNames] = useState<string[]>([]);
  const [activeTab, setActiveTab] = useState<string>(OBJECTIVEAI_TAB_ID);

  useEffect(() => {
    tauriInvoke<string[]>("list_plugins_with_viewer")
      .then((names) => { if (names) setPluginNames(names); })
      .catch((e) => {
        // eslint-disable-next-line no-console
        console.warn("list_plugins_with_viewer failed:", e);
      });
  }, []);

  const tabs: Tab[] = [
    { id: OBJECTIVEAI_TAB_ID, label: "ObjectiveAI" },
    ...pluginNames.map((name) => ({ id: name, label: name })),
  ];

  if (pluginNames.length === 0) {
    return <ObjectiveAIView />;
  }

  return (
    <div className="flex flex-col h-screen">
      <TabBar tabs={tabs} activeTab={activeTab} onSelect={setActiveTab} />
      <div className="flex-1 min-h-0 flex flex-col">
        {activeTab === OBJECTIVEAI_TAB_ID ? (
          <ObjectiveAIView />
        ) : (
          <PluginPane pluginName={activeTab} />
        )}
      </div>
    </div>
  );
}

export default App;

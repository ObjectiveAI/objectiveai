import { useState, useMemo, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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
import type { Entry } from "./types";

const KINDS: { kind: Entry["kind"]; label: string }[] = [
  { kind: "agent-completion", label: "Agent" },
  { kind: "execution", label: "Execution" },
  { kind: "invention", label: "Invention" },
  { kind: "laboratory", label: "Laboratory" },
];

function ObjectiveAIView() {
  const liveEntries = useEntries();
  const session = useSessionStorage(liveEntries, true);
  const [sessionPickerOpen, setSessionPickerOpen] = useState(false);

  const entries = session.restoredEntries ?? liveEntries;
  const { isCollapsed, toggle, collapseAll, expandAll } = useCollapseState(entries);
  const [activeKinds, setActiveKinds] = useState<Set<Entry["kind"]>>(
    new Set(KINDS.map((k) => k.kind))
  );

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

  const filtered = useMemo(
    () => entries.filter((e) => activeKinds.has(e.kind)),
    [entries, activeKinds],
  );

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
    <Shell statusBar={<StatusBar entries={entries} isHistorical={session.isViewingPast} />} banner={banner} entryCount={entries.length}>
      <SessionPicker
        open={sessionPickerOpen}
        onOpenChange={setSessionPickerOpen}
        sessions={session.pastSessions}
        currentSessionId={session.sessionId}
        onLoad={session.loadSession}
        onDelete={session.deleteSession}
      />
      {entries.length > 0 && (
        <div className="flex items-center gap-1.5 px-2 mb-4 select-none">
          {KINDS.map(({ kind, label }) => {
            const count = kindCounts.get(kind) ?? 0;
            const active = activeKinds.has(kind);
            return (
              <button
                key={kind}
                onClick={() => toggleKind(kind)}
                className={`px-2.5 py-1 rounded-sm font-mono text-[10px] transition-colors ${
                  active
                    ? "bg-copper-warm/20 text-copper-bright"
                    : "bg-ground-surface text-info-dim hover:text-info-mid"
                }`}
              >
                {label}
                {count > 0 && <span className="ml-1 opacity-70">({count})</span>}
              </button>
            );
          })}
          <div className="ml-auto flex gap-1">
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
        <div className="text-center text-info-dim italic py-12">
          Waiting for requests…
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
    invoke<string[]>("list_plugins_with_viewer")
      .then(setPluginNames)
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
    <div style={{ display: "flex", flexDirection: "column", height: "100vh" }}>
      <TabBar tabs={tabs} activeTab={activeTab} onSelect={setActiveTab} />
      <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
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

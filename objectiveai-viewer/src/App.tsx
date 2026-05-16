import { useState, useMemo, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useEntries } from "./hooks/useEntries";
import { Shell } from "./components/layout/Shell";
import { StatusBar } from "./components/layout/StatusBar";
import { EntryView } from "./components/views/EntryView";
import { TabBar, type Tab } from "./TabBar";
import { PluginPane } from "./PluginPane";
import type { Entry } from "./types";

const KINDS: { kind: Entry["kind"]; label: string }[] = [
  { kind: "agent-completion", label: "Agent" },
  { kind: "execution", label: "Execution" },
  { kind: "invention", label: "Invention" },
  { kind: "laboratory", label: "Laboratory" },
];

function ObjectiveAIView() {
  const entries = useEntries();
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

  return (
    <Shell statusBar={<StatusBar entries={entries} />} entryCount={entries.length}>
      {entries.length > 0 && (
        <div className="flex gap-1.5 px-2 mb-4 select-none">
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
        </div>
      )}

      {entries.length === 0 && (
        <div className="text-center text-info-dim italic py-12">
          Waiting for requests…
        </div>
      )}

      {filtered.map((entry) => (
        <EntryView key={entry.id} entry={entry} />
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

import { useState, useMemo } from "react";
import { useEntries } from "./hooks/useEntries";
import { Shell } from "./components/layout/Shell";
import { StatusBar } from "./components/layout/StatusBar";
import { EntryView } from "./components/views/EntryView";
import type { Entry } from "./types";

const KINDS: { kind: Entry["kind"]; label: string }[] = [
  { kind: "agent-completion", label: "Agent" },
  { kind: "execution", label: "Execution" },
  { kind: "invention", label: "Invention" },
  { kind: "laboratory", label: "Laboratory" },
];

function App() {
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

export default App;

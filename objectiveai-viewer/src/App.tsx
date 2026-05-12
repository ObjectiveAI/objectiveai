import { useEntries } from "./hooks/useEntries";
import { Shell } from "./components/layout/Shell";
import { StatusBar } from "./components/layout/StatusBar";
import { EntryView } from "./components/views/EntryView";

function App() {
  const entries = useEntries();

  return (
    <Shell statusBar={<StatusBar entries={entries} />}>
      {entries.length === 0 && (
        <div className="text-center text-info-dim italic py-12">
          Waiting for requests…
        </div>
      )}
      {entries.map((entry) => (
        <EntryView key={entry.id} entry={entry} />
      ))}
    </Shell>
  );
}

export default App;

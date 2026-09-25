import { useCallback, useEffect, useMemo, useState } from "react";
import type { AgentView } from "./bindings/AgentView";
import type { AppInfo } from "./bindings/AppInfo";
import type { TabKind } from "./bindings/TabKind";
import type { TabsSnapshot } from "./bindings/TabsSnapshot";
import { Rail } from "./components/Rail";
import { TabStrip } from "./components/TabStrip";
import { Empty } from "./components/ui";
import { SharedContext } from "./lib/context";
import { api } from "./lib/ipc";
import { Conversation } from "./screens/Conversation";
import { Files } from "./screens/Files";
import { Machines } from "./screens/Machines";
import { NewAgent } from "./screens/NewAgent";
import { Views } from "./screens/Views";
import { t } from "./strings";

// The daemon's list answers once and finishes — it is not live — so the
// rail re-asks on a short clock and after every action.
const RELIST_MS = 2500;

export function App() {
  const [agents, setAgents] = useState<AgentView[]>([]);
  const [listedAt, setListedAt] = useState(0);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [tabs, setTabs] = useState<TabsSnapshot>({ generation: 0, tabs: [], focused: null });

  const refreshAgents = useCallback(async () => {
    const listed = await api.agentsList();
    setAgents(listed.agents);
    setListedAt(Date.now());
  }, []);

  useEffect(() => {
    api.appInfo().then(setInfo);
    api.tabs().then(setTabs);
    refreshAgents();
    const timer = setInterval(refreshAgents, RELIST_MS);
    const unlisten = api.onTabs((snapshot) => setTabs((current) => (snapshot.generation >= current.generation ? snapshot : current)));
    return () => {
      clearInterval(timer);
      unlisten.then((stop) => stop());
    };
  }, [refreshAgents]);

  const apply = useCallback((snapshot: TabsSnapshot) => setTabs((current) => (snapshot.generation >= current.generation ? snapshot : current)), []);
  const open = useCallback((tab: TabKind) => void api.tabOpen(tab).then(apply), [apply]);
  const close = useCallback((key: string) => void api.tabClose(key).then(apply), [apply]);
  const focus = useCallback((key: string) => void api.tabFocus(key).then(apply), [apply]);

  const shared = useMemo(() => ({ agents, listedAt, refreshAgents, info, open, close }), [agents, listedAt, refreshAgents, info, open, close]);

  return (
    <SharedContext.Provider value={shared}>
      <div className="shell">
        <Rail focused={tabs.focused} />
        <main className="stage">
          <TabStrip snapshot={tabs} onFocus={focus} onClose={close} />
          <div className="panes">
            {tabs.tabs.length === 0 ? <Empty title={t.home.title} body={t.home.body} /> : null}
            {tabs.tabs.map(({ key, tab }) => (
              <div key={key} className="pane" hidden={tabs.focused !== key}>
                {tab.kind === "agent" ? <Conversation name={tab.name} tabKey={key} /> : null}
                {tab.kind === "new_agent" ? <NewAgent tabKey={key} /> : null}
                {tab.kind === "files" ? <Files /> : null}
                {tab.kind === "machines" ? <Machines /> : null}
                {tab.kind === "views" ? <Views /> : null}
              </div>
            ))}
          </div>
        </main>
      </div>
    </SharedContext.Provider>
  );
}

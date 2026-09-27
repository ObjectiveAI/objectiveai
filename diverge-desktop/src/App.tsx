import { useCallback, useEffect, useMemo, useState } from "react";
import type { AgentView } from "./bindings/AgentView";
import type { AppInfo } from "./bindings/AppInfo";
import type { CardView } from "./bindings/CardView";
import type { KnockView } from "./bindings/KnockView";
import type { SpaceSummary } from "./bindings/SpaceSummary";
import type { TabKind } from "./bindings/TabKind";
import type { TabsSnapshot } from "./bindings/TabsSnapshot";
import { Rail } from "./components/Rail";
import { TabStrip } from "./components/TabStrip";
import { Empty } from "./components/ui";
import { SharedContext } from "./lib/context";
import { api } from "./lib/ipc";
import { setMachineNames } from "./lib/format";
import { Conversation } from "./screens/Conversation";
import { Home } from "./screens/Home";
import { Inbox } from "./screens/Inbox";
import { Profile } from "./screens/Profile";
import { Machines } from "./screens/Machines";
import { NewAgent } from "./screens/NewAgent";
import { DoorPage } from "./screens/DoorPage";
import { Space } from "./screens/Space";
import { Spaces } from "./screens/Spaces";
import { Storage } from "./screens/Storage";
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
  const [spaces, setSpaces] = useState<SpaceSummary[]>([]);
  const [homeId, setHomeId] = useState<string | null>(null);
  const [knocks, setKnocks] = useState<KnockView[]>([]);
  const [cards, setCards] = useState<CardView[]>([]);
  const answerCard = useCallback(async (id: number, answer: string) => {
    await api.cardsAnswer(id, answer);
    setCards((c) => c.filter((x) => x.id !== id));
  }, []);

  const refreshSpaces = useCallback(async () => setSpaces(await api.spaces()), []);
  const answerKnock = useCallback(async (knockId: number, yes: boolean) => {
    await api.knocksAnswer(knockId, yes);
    setKnocks((k) => k.filter((x) => x.knock_id !== knockId));
    refreshSpaces();
  }, [refreshSpaces]);

  const refreshAgents = useCallback(async () => {
    const listed = await api.agentsList();
    setAgents(listed.agents);
    setListedAt(Date.now());
  }, []);

  useEffect(() => {
    api.appInfo().then(setInfo);
    api.spacesHome().then(setHomeId);
    api.machineNames().then(setMachineNames);
    api.tabs().then((snapshot) => {
      setTabs(snapshot);
      // Lead with the work: Home is the first thing open.
      if (snapshot.tabs.length === 0) api.tabOpen({ kind: "home" }).then(apply);
    });
    refreshAgents();
    refreshSpaces();
    const knocksScope = api.knocksWatch((event) => {
      if (event.event === "knock") setKnocks((k) => (k.some((x) => x.knock_id === event.knock.knock_id) ? k : [...k, event.knock]));
    });
    const cardsScope = api.cardsWatch((event) => {
      if (event.event === "card") setCards((c) => (c.some((x) => x.id === event.card.id) ? c : [...c, event.card]));
      else if (event.event === "answered") setCards((c) => c.filter((x) => x.id !== event.id));
    });
    const timer = setInterval(refreshAgents, RELIST_MS);
    const unlisten = api.onTabs((snapshot) => setTabs((current) => (snapshot.generation >= current.generation ? snapshot : current)));
    return () => {
      clearInterval(timer);
      unlisten.then((stop) => stop());
      knocksScope.then((id) => api.scopeClose(id));
      cardsScope.then((id) => api.scopeClose(id));
    };
  }, [refreshAgents, refreshSpaces]);

  const apply = useCallback((snapshot: TabsSnapshot) => setTabs((current) => (snapshot.generation >= current.generation ? snapshot : current)), []);
  const open = useCallback((tab: TabKind) => void api.tabOpen(tab).then(apply), [apply]);
  const close = useCallback((key: string) => void api.tabClose(key).then(apply), [apply]);
  const focus = useCallback((key: string) => void api.tabFocus(key).then(apply), [apply]);

  // Keyboard: the menu carries ⌘N/⌘W/⌘1/⌘2 in the app; the page handles
  // ⌘3–9 (tabs by position) and ⌘[ / ⌘] (previous / next), and stands in
  // for the menu in the browser preview.
  useEffect(() => {
    const act = (id: string) => {
      if (id === "new-agent") open({ kind: "new_agent" });
      else if (id === "close-tab" && tabs.focused) close(tabs.focused);
      else if (id === "go-home") open({ kind: "home" });
      else if (id === "go-inbox") open({ kind: "inbox" });
    };
    const unlisten = api.onMenu(act);
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      const inApp = "__TAURI_INTERNALS__" in window;
      if (!inApp && e.key === "n") { e.preventDefault(); act("new-agent"); }
      else if (!inApp && e.key === "w") { e.preventDefault(); act("close-tab"); }
      else if (!inApp && e.key === "1") { e.preventDefault(); act("go-home"); }
      else if (!inApp && e.key === "2") { e.preventDefault(); act("go-inbox"); }
      else if (/^[3-9]$/.test(e.key)) {
        const tab = tabs.tabs[Number(e.key) - 3];
        if (tab) { e.preventDefault(); focus(tab.key); }
      } else if (e.key === "]" || e.key === "[") {
        const at = tabs.tabs.findIndex((t) => t.key === tabs.focused);
        const next = tabs.tabs[(at + (e.key === "]" ? 1 : tabs.tabs.length - 1)) % Math.max(1, tabs.tabs.length)];
        if (next) { e.preventDefault(); focus(next.key); }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      unlisten.then((stop) => stop());
    };
  }, [tabs, open, close, focus]);

  // Whether your keys file could be read: if not, nothing is signed, and the app says so everywhere.
  const [keysBroken, setKeysBroken] = useState<string | null>(null);
  useEffect(() => {
    api.identityBroken().then(setKeysBroken).catch(() => setKeysBroken(null));
  }, []);

  const shared = useMemo(
    () => ({ agents, listedAt, refreshAgents, info, open, close, spaces, homeId, refreshSpaces, knocks, answerKnock, cards, answerCard }),
    [agents, listedAt, refreshAgents, info, open, close, spaces, homeId, refreshSpaces, knocks, answerKnock, cards, answerCard],
  );

  return (
    <SharedContext.Provider value={shared}>
      <div className="shell">
        <Rail focused={tabs.focused} />
        <main className="stage">
          <TabStrip snapshot={tabs} onFocus={focus} onClose={close} />
          {keysBroken ? (
            <div className="banner banner-warn">
              <strong>{t.keys.unreadable}</strong> {t.keys.where} <span className="mono selectable">{keysBroken}</span>. {t.keys.untouched}
            </div>
          ) : null}
          <div className="panes">
            {tabs.tabs.length === 0 ? <Empty title={t.emptyState.title} body={t.emptyState.body} /> : null}
            {tabs.tabs.map(({ key, tab }) => (
              <div key={key} className="pane" hidden={tabs.focused !== key}>
                {tab.kind === "agent" ? <Conversation name={tab.name} tabKey={key} /> : null}
                {tab.kind === "new_agent" ? <NewAgent tabKey={key} /> : null}
                {tab.kind === "storage" ? <Storage /> : null}
                {tab.kind === "home" ? <Home /> : null}
                {tab.kind === "inbox" ? <Inbox /> : null}
                {tab.kind === "profile" ? <Profile /> : null}
                {tab.kind === "spaces" ? <Spaces /> : null}
                {tab.kind === "space" ? <Space id={tab.id} tabKey={key} /> : null}
                {tab.kind === "door" ? <DoorPage invite={tab.invite} tabKey={key} /> : null}
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

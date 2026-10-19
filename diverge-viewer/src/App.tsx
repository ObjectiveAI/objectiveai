import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { afterAnswer, afterCardEvent, arrivalKeys, arrivals, noCards } from "./lib/cards";
import type { AgentView } from "./bindings/AgentView";
import type { AppInfo } from "./bindings/AppInfo";
import type { KnockView } from "./bindings/KnockView";
import type { FileNoticeView } from "./bindings/FileNoticeView";
import type { KeysBrokenView } from "./bindings/KeysBrokenView";
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
  // Cards waiting on you, and those withdrawn because their agent stopped waiting.
  const [{ cards, withdrawn }, setCards] = useState(noCards);
  const answerCard = useCallback(async (id: number, answer: string) => {
    // A card answered or withdrawn meanwhile takes no answer: it stays until the app's event says which.
    const taken = await api.cardsAnswer(id, answer).then(() => true, () => false);
    setCards((s) => afterAnswer(s, id, taken));
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

  // Files the app couldn't use, set aside untouched or left where they are: said on every screen, as they turn up.
  const [setAside, setSetAside] = useState<FileNoticeView[]>([]);
  const refreshSetAside = useCallback(() => {
    api.filesSetAside().then(setSetAside).catch(() => setSetAside([]));
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
    refreshSetAside();
    const knocksScope = api.knocksWatch((event) => {
      if (event.event === "knock") setKnocks((k) => (k.some((x) => x.knock_id === event.knock.knock_id) ? k : [...k, event.knock]));
    });
    const cardsScope = api.cardsWatch((event) => setCards((s) => afterCardEvent(s, event)));
    const timer = setInterval(() => {
      refreshAgents();
      refreshSetAside();
    }, RELIST_MS);
    const unlisten = api.onTabs((snapshot) => setTabs((current) => (snapshot.generation >= current.generation ? snapshot : current)));
    return () => {
      clearInterval(timer);
      unlisten.then((stop) => stop());
      knocksScope.then((id) => api.scopeClose(id));
      cardsScope.then((id) => api.scopeClose(id));
    };
  }, [refreshAgents, refreshSpaces, refreshSetAside]);

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

  // When a tab opens, focus goes to its page's heading, once the page has one (a room's comes when it's read).
  const lastFocused = useRef<string | null>(null);
  useEffect(() => {
    const key = tabs.focused;
    if (!key || key === lastFocused.current) return;
    lastFocused.current = key;
    let frame = 0;
    let tries = 0;
    const find = () => {
      const pane = document.querySelector<HTMLElement>(`.pane[data-pane="${CSS.escape(key)}"]`);
      const heading = pane && !pane.hidden ? pane.querySelector<HTMLElement>("h1") : null;
      if (!heading) {
        if (tries++ < 90) frame = requestAnimationFrame(find);
        return;
      }
      // Someone already somewhere in the page keeps their place.
      if (pane!.contains(document.activeElement)) return;
      if (!heading.hasAttribute("tabindex")) heading.setAttribute("tabindex", "-1");
      heading.focus({ preventScroll: true });
    };
    find();
    return () => cancelAnimationFrame(frame);
  }, [tabs.focused]);

  // Cards and knocks, said aloud as they arrive (politely: after whatever is being read).
  const [heard, setHeard] = useState<{ seen: Set<string>; said: string }>({ seen: new Set(), said: "" });
  useEffect(() => {
    setHeard((h) => {
      const fresh = arrivals(h.seen, cards, knocks);
      return fresh.length ? { seen: new Set([...h.seen, ...arrivalKeys(cards, knocks)]), said: fresh.join(". ") } : h;
    });
  }, [cards, knocks]);

  // Whether your keys file could be read: if not, nothing is signed, and the app says so everywhere.
  const [keysBroken, setKeysBroken] = useState<KeysBrokenView | null>(null);
  useEffect(() => {
    api.identityBroken().then(setKeysBroken).catch(() => setKeysBroken(null));
  }, []);

  const shared = useMemo(
    () => ({ agents, listedAt, refreshAgents, info, open, close, spaces, homeId, refreshSpaces, knocks, answerKnock, cards, withdrawn, answerCard }),
    [agents, listedAt, refreshAgents, info, open, close, spaces, homeId, refreshSpaces, knocks, answerKnock, cards, withdrawn, answerCard],
  );

  return (
    <SharedContext.Provider value={shared}>
      <div className="shell">
        <Rail focused={tabs.focused} />
        <div className="visually-hidden" role="status" aria-live="polite">{heard.said}</div>
        <main className="stage">
          <TabStrip snapshot={tabs} onFocus={focus} onClose={close} />
          {info && info.folder_held.state !== "yes" ? (
            <div className="banner banner-warn"><span className="banner-text">
              <strong>{info.folder_held.state === "in_use" ? t.folder.inUse : t.folder.unchecked}</strong> {t.folder.where}{" "}
              <span className="mono selectable">{info.folder}</span>.
              {info.folder_held.state === "unchecked" ? <> {t.folder.systemSaid} <span className="mono selectable">{info.folder_held.error}</span>.</> : null}
            </span></div>
          ) : info && !info.network ? (
            <div className="banner banner-warn">
              <strong>{t.network.absent}</strong>
            </div>
          ) : null}
          {keysBroken ? (
            <div className="banner banner-warn"><span className="banner-text">
              <strong>{keysBroken.newer ? t.keys.newer : t.keys.unreadable}</strong> {t.keys.where}{" "}
              <span className="mono selectable">{keysBroken.file}</span>. {t.keys.untouched}
            </span></div>
          ) : null}
          {setAside.map((f) => (
            <div key={f.kept_as ?? f.file} className="banner banner-warn"><span className="banner-text">
              <strong>
                {f.room ? `${t.files.recordOf} ${f.room}` : (t.files.kinds[f.kind] ?? f.kind)}
                {f.last_good_copy ? ` ${t.files.lastGoodCopy}` : ""}
              </strong>{" "}
              {f.why === "refused" && f.kind === "record copy" ? t.files.refusedRecord : t.files.why[f.why]}
              {f.kept_as ? <>, {t.files.setAsideAs} <span className="mono selectable">{f.kept_as}</span>.</> : <>, {t.files.leftInPlace}</>}
              {f.error ? <> {t.files.systemSaid} <span className="mono selectable">{f.error}</span>.</> : null} {t.files.carriedOn[f.carried_on]}{" "}
              {t.files.untouched}
            </span></div>
          ))}
          <div className="panes">
            {tabs.tabs.length === 0 ? <Empty title={t.emptyState.title} body={t.emptyState.body} /> : null}
            {tabs.tabs.map(({ key, tab }) => (
              <div key={key} className="pane" data-pane={key} hidden={tabs.focused !== key}>
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

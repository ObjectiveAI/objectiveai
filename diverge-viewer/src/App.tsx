import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { AppInfo } from "./bindings/AppInfo";
import type { FileNoticeView } from "./bindings/FileNoticeView";
import type { TabKind } from "./bindings/TabKind";
import type { TabsSnapshot } from "./bindings/TabsSnapshot";
import { Rail } from "./components/Rail";
import { TabStrip } from "./components/TabStrip";
import { Empty } from "./components/ui";
import type { WireStatus } from "./bindings/WireStatus";
import { SharedContext } from "./lib/context";
import { app, errorText } from "./lib/app";
import { link, useAgents } from "./lib/daemon";
import { Conversation } from "./screens/Conversation";
import { Machines } from "./screens/Machines";
import { NewAgent } from "./screens/NewAgent";
import { Settings } from "./screens/Settings";
import { Storage } from "./screens/Storage";
import { Views } from "./screens/Views";
import { t } from "./strings";

export function App() {
  // The link, as Rust last said; asked once at start, and again when a
  // list ends, since a list that ends may be a connection that went.
  const [status, setStatus] = useState<WireStatus>({ state: "idle" });
  const refreshLink = useCallback(() => {
    link.status().then(setStatus).catch((error) => setStatus({ state: "failed", message: errorText(error) }));
  }, []);
  const redial = useCallback(() => {
    setStatus({ state: "dialling" });
    link.reset().then(refreshLink).catch((error) => setStatus({ state: "failed", message: errorText(error) }));
  }, [refreshLink]);
  useEffect(refreshLink, [refreshLink]);
  const connected = status.state === "connected";
  // The agents, as the daemon lists them, held open while the link is up.
  const agents = useAgents(connected);
  useEffect(() => {
    if (agents.ended) refreshLink();
  }, [agents.ended, refreshLink]);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [tabs, setTabs] = useState<TabsSnapshot>({ generation: 0, tabs: [], focused: null });

  // Files the app couldn't use, set aside untouched or left where they are: said on every screen, as they turn up.
  const [setAside, setSetAside] = useState<FileNoticeView[]>([]);
  const refreshSetAside = useCallback(() => {
    app.filesSetAside().then(setSetAside).catch(() => setSetAside([]));
  }, []);

  const apply = useCallback((snapshot: TabsSnapshot) => setTabs((current) => (snapshot.generation >= current.generation ? snapshot : current)), []);

  useEffect(() => {
    app.info().then(setInfo).catch(() => setInfo(null));
    app.tabs().then(apply).catch(() => {});
    refreshSetAside();
    const unlisten = app.onTabs(apply);
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [apply, refreshSetAside]);

  const open = useCallback((tab: TabKind) => void app.tabOpen(tab).then(apply), [apply]);
  const close = useCallback((key: string) => void app.tabClose(key).then(apply), [apply]);
  const focus = useCallback((key: string) => void app.tabFocus(key).then(apply), [apply]);

  // Keyboard: the menu carries ⌘N/⌘W/⌘, in the app; the page handles
  // ⌘3–9 (tabs by position) and ⌘[ / ⌘] (previous / next), and stands in
  // for the menu in the browser preview.
  useEffect(() => {
    const act = (id: string) => {
      if (id === "new-agent") open({ kind: "new_agent" });
      else if (id === "close-tab" && tabs.focused) close(tabs.focused);
      else if (id === "go-settings") open({ kind: "settings" });
    };
    const unlisten = app.onMenu(act);
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      const inApp = "__TAURI_INTERNALS__" in window;
      if (!inApp && e.key === "n") { e.preventDefault(); act("new-agent"); }
      else if (!inApp && e.key === "w") { e.preventDefault(); act("close-tab"); }
      else if (!inApp && e.key === ",") { e.preventDefault(); act("go-settings"); }
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
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [tabs, open, close, focus]);

  // When a tab opens, focus goes to its page's heading, once the page has one.
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

  const shared = useMemo(() => ({ agents, connected, link: status, redial, info, open, close }), [agents, connected, status, redial, info, open, close]);

  return (
    <SharedContext.Provider value={shared}>
      <div className="shell">
        <Rail focused={tabs.focused} />
        <main className="stage">
          <TabStrip snapshot={tabs} onFocus={focus} onClose={close} />
          {info && info.folder_held.state !== "yes" ? (
            <div className="banner banner-warn"><span className="banner-text">
              <strong>{info.folder_held.state === "in_use" ? t.folder.inUse : t.folder.unchecked}</strong> {t.folder.where}{" "}
              <span className="mono selectable">{info.folder}</span>.
              {info.folder_held.state === "unchecked" ? <> {t.folder.systemSaid} <span className="mono selectable">{info.folder_held.error}</span>.</> : null}
            </span></div>
          ) : null}
          {setAside.map((f) => (
            <div key={f.kept_as ?? f.file} className="banner banner-warn"><span className="banner-text">
              <strong>
                {t.files.kinds[f.kind] ?? f.kind}
                {f.last_good_copy ? ` ${t.files.lastGoodCopy}` : ""}
              </strong>{" "}
              {t.files.why[f.why]}
              {f.kept_as ? <>, {t.files.setAsideAs} <span className="mono selectable">{f.kept_as}</span>.</> : <>, {t.files.leftInPlace}</>}
              {f.error ? <> {t.files.systemSaid} <span className="mono selectable">{f.error}</span>.</> : null} {t.files.carriedOn[f.carried_on]}{" "}
              {t.files.untouched}
            </span></div>
          ))}
          <div className="panes">
            {tabs.tabs.length === 0 ? <Empty title={t.emptyState.title} body={t.emptyState.body} /> : null}
            {tabs.tabs.map(({ key, tab }) => (
              <div key={key} className="pane" data-pane={key} hidden={tabs.focused !== key}>
                {tab.kind === "agent" ? <Conversation template={tab.template} index={tab.index} name={tab.name} /> : null}
                {tab.kind === "new_agent" ? <NewAgent /> : null}
                {tab.kind === "storage" ? <Storage /> : null}
                {tab.kind === "machines" ? <Machines /> : null}
                {tab.kind === "views" ? <Views /> : null}
                {tab.kind === "settings" ? <Settings /> : null}
              </div>
            ))}
          </div>
        </main>
      </div>
    </SharedContext.Provider>
  );
}

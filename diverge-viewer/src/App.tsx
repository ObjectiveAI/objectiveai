import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { AppInfo } from "./bindings/AppInfo";
import type { FileNoticeView } from "./bindings/FileNoticeView";
import type { TabKind } from "./bindings/TabKind";
import type { TabsSnapshot } from "./bindings/TabsSnapshot";
import { Rail } from "./components/Rail";
import { Empty } from "./components/ui";
import { SharedContext } from "./lib/context";
import { app, errorText } from "./lib/app";
import { type WireStatus, link, useAgents } from "./lib/daemon";
import { Conversation } from "./screens/Conversation";
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

  // Keyboard: the window has no menu and no tab strip, so the page
  // handles the two shortcuts there are: ⌘N new agent, ⌘, settings.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      if (e.key === "n") { e.preventDefault(); open({ kind: "new_agent" }); }
      else if (e.key === ",") { e.preventDefault(); open({ kind: "settings" }); }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  // When a page opens, focus goes to its heading, once the page has one.
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

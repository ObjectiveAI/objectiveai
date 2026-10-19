import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { opens, rove } from "../lib/keys";
import { spaceTitle as nameOf } from "../lib/format";
import type { TabKind } from "../bindings/TabKind";
import type { TabsSnapshot } from "../bindings/TabsSnapshot";
import { useShared } from "../lib/context";
import { t } from "../strings";
import { Button, Dot, Icon } from "./ui";

/** An invite's title, for its tab only: the app reads invites properly in Rust. */
function inviteTitle(text: string): string {
  try {
    const body = text.replace(/^diverge-invite:/, "").replace(/-/g, "+").replace(/_/g, "/");
    const json = JSON.parse(decodeURIComponent(escape(atob(body + "===".slice((body.length + 3) % 4)))));
    return String(json.title ?? "");
  } catch {
    return "";
  }
}

function title(tab: TabKind): string {
  switch (tab.kind) {
    case "agent":
      return tab.name;
    case "new_agent":
      return t.create.title;
    case "storage":
      return t.storage.title;
    case "machines":
      return t.machines.title;
    case "views":
      return t.views.title;
    case "home":
      return t.home.title;
    case "inbox":
      return t.inbox.title;
    case "profile":
      return t.profile.title;
    case "spaces":
      return t.spaces.title;
    case "space":
      return spaceTitle(tab.id);
    case "door":
      return `${t.door.fromDm}: ${inviteTitle(tab.invite)}`;
  }
}

let spaceTitles: Record<string, string> = {};
function spaceTitle(id: string): string {
  return spaceTitles[id] ?? id;
}

export function TabStrip(props: { snapshot: TabsSnapshot; onFocus: (key: string) => void; onClose: (key: string) => void }) {
  const { agents, spaces } = useShared();
  spaceTitles = Object.fromEntries(spaces.map((s) => [s.id, nameOf(s)]));
  const { tabs, focused } = props.snapshot;
  // Roving focus: Tab enters the strip once, at the tab you're on; arrow keys, Home and End move along it,
  // and Enter or Space opens the tab they're on.
  const [rover, setRover] = useState<string | null>(focused);
  useEffect(() => setRover(focused), [focused]);
  const stop = tabs.some((x) => x.key === rover) ? rover : (focused ?? tabs[0]?.key ?? null);
  // Many tabs scroll sideways; the one you're on is always in view.
  const strip = useRef<HTMLDivElement>(null);
  useEffect(() => {
    strip.current?.querySelector(".tab.on")?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [focused, tabs.length]);
  const onKey = (e: KeyboardEvent<HTMLDivElement>, key: string) => {
    if (e.target !== e.currentTarget) return; // keys on the close button are the button's own
    if (opens(e.key)) {
      e.preventDefault();
      props.onFocus(key);
      return;
    }
    const to = rove(tabs.length, tabs.findIndex((x) => x.key === key), e.key);
    if (to === null) return;
    e.preventDefault();
    const next = tabs[to].key;
    setRover(next);
    strip.current?.querySelector<HTMLElement>(`[data-tab="${CSS.escape(next)}"]`)?.focus();
  };
  return (
    <div className="strip" role="tablist" aria-orientation="horizontal" ref={strip}>
      {tabs.map(({ key, tab }) => {
        const agent = tab.kind === "agent" ? agents.find((a) => a.name === tab.name) : undefined;
        const name = title(tab);
        return (
          <div
            key={key}
            data-tab={key}
            role="tab"
            tabIndex={stop === key ? 0 : -1}
            aria-selected={focused === key}
            className={`tab${focused === key ? " on" : ""}`}
            title={name}
            onMouseDown={() => props.onFocus(key)}
            onKeyDown={(e) => onKey(e, key)}
            // The browser shows a focused tab only as far as it already peeks out of the strip: show all of it.
            onFocus={(e) => e.currentTarget.scrollIntoView({ block: "nearest", inline: "nearest" })}
          >
            {agent ? <Dot state={agent.active ? "working" : agent.last_active ? "idle" : "never"} /> : null}
            <span className="tab-title">{name}</span>
            <Button kind="icon" label={`${t.common.close} ${name}`} onMouseDown={(e) => e.stopPropagation()} onClick={() => props.onClose(key)}>
              <Icon name="close" />
            </Button>
          </div>
        );
      })}
    </div>
  );
}

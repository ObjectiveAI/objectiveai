import type { TabKind } from "../bindings/TabKind";
import type { TabsSnapshot } from "../bindings/TabsSnapshot";
import { useShared } from "../lib/context";
import { t } from "../strings";
import { Dot, Icon } from "./ui";

function title(tab: TabKind): string {
  switch (tab.kind) {
    case "agent":
      return tab.name;
    case "new_agent":
      return t.create.title;
    case "files":
      return t.files.title;
    case "machines":
      return t.machines.title;
    case "views":
      return t.views.title;
  }
}

export function TabStrip(props: { snapshot: TabsSnapshot; onFocus: (key: string) => void; onClose: (key: string) => void }) {
  const { agents } = useShared();
  return (
    <div className="strip" role="tablist">
      {props.snapshot.tabs.map(({ key, tab }) => {
        const agent = tab.kind === "agent" ? agents.find((a) => a.name === tab.name) : undefined;
        return (
          <div key={key} role="tab" aria-selected={props.snapshot.focused === key} className={`tab${props.snapshot.focused === key ? " on" : ""}`} onMouseDown={() => props.onFocus(key)}>
            {agent ? <Dot state={agent.active ? "working" : agent.last_active ? "idle" : "never"} /> : null}
            <span className="tab-title">{title(tab)}</span>
            <button className="tab-close" title={t.common.close} onMouseDown={(e) => e.stopPropagation()} onClick={() => props.onClose(key)}>
              <Icon name="close" />
            </button>
          </div>
        );
      })}
    </div>
  );
}

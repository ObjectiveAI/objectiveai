import { agentKey, useShared } from "../lib/context";
import { t } from "../strings";
import wordmark from "../assets/wordmark.svg";
import { Button, Dot, Icon, Row } from "./ui";

export function Rail(props: { focused: string | null }) {
  const { agents, connected, info, open } = useShared();
  const here = (key: string) => props.focused === key;
  return (
    <nav className="rail">
      <div className="rail-body">
        <div className="rail-brand">
          <img className="brand-wordmark" src={wordmark} alt={t.app.name} draggable={false} />
          <span className="brand-draft">{t.app.draft}</span>
        </div>

        <div className="rail-section">
          <div className="rail-head">
            <span>{t.rail.agents}</span>
          </div>
          {!connected ? <p className="rail-empty muted">{t.daemon.notConnectedShort}</p> : agents.listed && agents.items.length === 0 ? <p className="rail-empty muted">{t.rail.empty}</p> : null}
          <ul className="rail-list">
            {agents.items.map((a) => {
              const key = agentKey(a);
              const state = a.active ? "working" : a.last_active ? "idle" : "never";
              const name = a.name ?? `${t.rail.unnamed} ${a.index}`;
              return (
                <li key={key}>
                  <Row className="rail-item" on={here(key)} onClick={() => open({ kind: "agent", template: a.template, index: a.index, name: a.name ?? null })} title={a.template}>
                    <Dot state={state} />
                    <span className="rail-item-name">{name}</span>
                  </Row>
                </li>
              );
            })}
          </ul>
          <Button kind="secondary" small onClick={() => open({ kind: "new_agent" })}>
            <Icon name="plus" /> {t.rail.newAgent}
          </Button>
        </div>

        <div className="rail-section rail-places">
          <Row className="rail-item" on={here("storage")} onClick={() => open({ kind: "storage" })}>
            <Icon name="files" /> <span className="rail-item-name">{t.rail.storage}</span>
          </Row>
          <Row className="rail-item" on={here("machines")} onClick={() => open({ kind: "machines" })}>
            <Icon name="machine" /> <span className="rail-item-name">{t.rail.machines}</span>
          </Row>
          <Row className="rail-item" on={here("views")} onClick={() => open({ kind: "views" })}>
            <Icon name="view" /> <span className="rail-item-name">{t.rail.views}</span>
          </Row>
          <Row className="rail-item" on={here("settings")} onClick={() => open({ kind: "settings" })}>
            <Icon name="home" /> <span className="rail-item-name">{t.rail.settings}</span>
          </Row>
        </div>
      </div>

      {info && info.folder_held.state !== "yes" ? (
        <div className="stand-in-line" title={info.folder_held.state === "in_use" ? t.folder.inUse : t.folder.unchecked}>
          <span>{info.folder_held.state === "in_use" ? t.folder.inUseShort : t.folder.uncheckedShort}</span>
        </div>
      ) : !connected ? (
        <div className="stand-in-line" title={t.daemon.notConnected}>
          <span>{t.daemon.notConnectedShort}</span>
        </div>
      ) : agents.ended ? (
        <div className="stand-in-line" title={agents.ended}>
          <span>{t.daemon.listEndedShort}</span>
        </div>
      ) : null}
    </nav>
  );
}

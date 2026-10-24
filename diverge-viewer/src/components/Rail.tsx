import { useShared } from "../lib/context";
import { t } from "../strings";
import wordmark from "../assets/wordmark.svg";
import { Icon, Row } from "./ui";

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

        <div className="rail-section rail-places">
          <Row className="rail-item" on={here("graph")} onClick={() => open({ kind: "graph" })}>
            <Icon name="graph" /> <span className="rail-item-name">{t.rail.graph}</span>
          </Row>
          <Row className="rail-item" on={here("agents")} onClick={() => open({ kind: "agents" })}>
            <Icon name="agent" /> <span className="rail-item-name">{t.rail.agents}</span>
          </Row>
          <Row className="rail-item" on={here("tools")} onClick={() => open({ kind: "tools" })}>
            <Icon name="tool" /> <span className="rail-item-name">{t.rail.tools}</span>
          </Row>
          <Row className="rail-item" on={here("storage")} onClick={() => open({ kind: "storage" })}>
            <Icon name="files" /> <span className="rail-item-name">{t.rail.storage}</span>
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

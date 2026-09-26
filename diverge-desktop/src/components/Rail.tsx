import { useShared } from "../lib/context";
import { kindTitle, providerName } from "../lib/format";
import { t } from "../strings";
import wordmark from "../assets/wordmark.svg";
import { Dot, Icon } from "./ui";

export function Rail(props: { focused: string | null }) {
  const { agents, info, open } = useShared();
  const item = (key: string) => (props.focused === key ? " on" : "");
  return (
    <nav className="rail">
      <div className="rail-brand">
        <img className="brand-wordmark" src={wordmark} alt={t.app.name} draggable={false} />
        <span className="brand-draft">{t.app.draft}</span>
      </div>

      <div className="rail-section">
        <div className="rail-head">
          <span>{t.rail.agents}</span>
        </div>
        {agents.length === 0 ? <p className="rail-empty muted">{t.rail.empty}</p> : null}
        <ul className="rail-list">
          {agents.map((a) => {
            const state = a.active ? "working" : a.last_active ? "idle" : "never";
            return (
              <li key={a.name}>
                <button className={`rail-item${item(`agent:${a.name}`)}`} onClick={() => open({ kind: "agent", name: a.name })} title={a.provider ? `${kindTitle(a.image_name)} · ${providerName(a.provider)}` : kindTitle(a.image_name)}>
                  <Dot state={state} />
                  <span className="rail-item-name">{a.name}</span>
                  <span className="rail-item-kind">{kindTitle(a.image_name)}</span>
                </button>
              </li>
            );
          })}
        </ul>
        <button className="rail-new" onClick={() => open({ kind: "new_agent" })}>
          <Icon name="plus" /> {t.rail.newAgent}
        </button>
      </div>

      <div className="rail-section rail-places">
        <button className={`rail-item${item("files")}`} onClick={() => open({ kind: "files" })}>
          <Icon name="files" /> <span className="rail-item-name">{t.rail.files}</span>
        </button>
        <button className={`rail-item${item("machines")}`} onClick={() => open({ kind: "machines" })}>
          <Icon name="machine" /> <span className="rail-item-name">{t.rail.machines}</span>
        </button>
        <button className={`rail-item${item("views")}`} onClick={() => open({ kind: "views" })}>
          <Icon name="view" /> <span className="rail-item-name">{t.rail.views}</span>
        </button>
      </div>

      {info?.stand_in ? (
        <div className="stand-in" title={`${t.standIn.pin} ${info.contract_pin}`}>
          <span className="stand-in-badge">{t.standIn.badge}</span>
          <span className="stand-in-note">{info.stand_in_host === "browser-preview" ? t.standIn.preview : t.standIn.note}</span>
          <span className="stand-in-pin mono">{t.standIn.pin} {info.contract_pin.slice(0, 9)}</span>
        </div>
      ) : null}
    </nav>
  );
}

import { useShared } from "../lib/context";
import { kindTitle, providerName, spaceTitle } from "../lib/format";
import { t } from "../strings";
import wordmark from "../assets/wordmark.svg";
import { Button, Dot, Icon, Row } from "./ui";

export function Rail(props: { focused: string | null }) {
  const { agents, info, open, spaces, knocks, cards } = useShared();
  const here = (key: string) => props.focused === key;
  return (
    <nav className="rail">
      <div className="rail-body">
        <div className="rail-brand">
          <img className="brand-wordmark" src={wordmark} alt={t.app.name} draggable={false} />
          <span className="brand-draft">{t.app.draft}</span>
        </div>

        <div className="rail-section">
          <Row className="rail-item rail-home" on={here("home")} onClick={() => open({ kind: "home" })}>
            <Icon name="home" /> <span className="rail-item-name">{t.rail.home}</span>
          </Row>
          <Row className="rail-item rail-home" on={here("inbox")} onClick={() => open({ kind: "inbox" })}>
            <Icon name="agent" /> <span className="rail-item-name">{t.rail.inbox}</span>
            {cards.length + knocks.length ? <span className="rail-count">{cards.length + knocks.length}</span> : null}
          </Row>
          <Row className="rail-item rail-home" on={here("profile")} onClick={() => open({ kind: "profile" })}>
            <Icon name="agent" /> <span className="rail-item-name">{t.rail.you}</span>
          </Row>
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
                  <Row className="rail-item" on={here(`agent:${a.name}`)} onClick={() => open({ kind: "agent", name: a.name })} title={a.provider ? `${kindTitle(a.image_name)} · ${providerName(a.provider)}` : kindTitle(a.image_name)}>
                    <Dot state={state} />
                    <span className="rail-item-name">{a.name}</span>
                    {cards.some((c) => c.agent === a.name) ? <span className="rail-count">{t.cards.railTag}</span> : <span className="rail-item-kind">{kindTitle(a.image_name)}</span>}
                  </Row>
                </li>
              );
            })}
          </ul>
          <Button kind="secondary" small onClick={() => open({ kind: "new_agent" })}>
            <Icon name="plus" /> {t.rail.newAgent}
          </Button>
        </div>

        <div className="rail-section">
          <div className="rail-head">
            <span>{t.rail.spaces}</span>
          </div>
          <ul className="rail-list">
            {spaces.map((s) => (
              <li key={s.id}>
                <Row className="rail-item" on={here(`space:${s.id}`)} onClick={() => open({ kind: "space", id: s.id })} title={s.mine ? t.spaces.youHostIt : `${t.spaces.hostedBy} ${providerName(s.host)}`}>
                  <Dot state={s.online ? "idle" : "never"} />
                  <span className="rail-item-name">{spaceTitle(s)}</span>
                  <span className="rail-item-kind">{t.spaces.kinds[s.kind] ?? s.kind}</span>
                </Row>
              </li>
            ))}
          </ul>
          <Row className="rail-item rail-all" on={here("spaces")} onClick={() => open({ kind: "spaces" })}>
            <Icon name="view" /> {t.rail.allSpaces}
            {knocks.length ? <span className="rail-count" title={t.spaces.door}>{knocks.length} {t.rail.atTheDoor}</span> : null}
          </Row>
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
        </div>
      </div>

      {info?.stand_in ? (
        <div className="stand-in-line" title={`${info.stand_in_host === "browser-preview" ? t.standIn.preview : t.standIn.note} ${t.standIn.pin} ${info.contract_pin}`}>
          <span>{info.stand_in_host === "browser-preview" ? t.standIn.previewShort : t.standIn.label}</span>
          <span className="mono">{info.contract_pin.slice(0, 9)}</span>
        </div>
      ) : null}
      {info && info.folder_held.state !== "yes" ? (
        <div className="stand-in-line" title={info.folder_held.state === "in_use" ? t.folder.inUse : t.folder.unchecked}>
          <span>{info.folder_held.state === "in_use" ? t.folder.inUseShort : t.folder.uncheckedShort}</span>
          <span className="mono">{info.contract_pin.slice(0, 9)}</span>
        </div>
      ) : info && !info.network ? (
        <div className="stand-in-line" title={t.network.absent}>
          <span>{t.network.absentShort}</span>
          <span className="mono">{info.contract_pin.slice(0, 9)}</span>
        </div>
      ) : null}
    </nav>
  );
}

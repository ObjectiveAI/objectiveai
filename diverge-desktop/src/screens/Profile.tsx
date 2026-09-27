import { useEffect, useState } from "react";
import type { ProfileView } from "../bindings/ProfileView";
import { Markdown } from "../components/Markdown";
import { Button, Chip, Empty, Section } from "../components/ui";
import { useShared } from "../lib/context";
import { ago, bytes, kindTitle, providerName, providerWay, time } from "../lib/format";
import { api } from "../lib/ipc";
import { t } from "../strings";

export function Profile() {
  const { open } = useShared();
  const [p, setP] = useState<ProfileView | null>(null);
  useEffect(() => {
    api.profile().then(setP);
  }, []);
  if (!p) return <Empty title={t.profile.title} />;
  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.profile.title}</h1>
        <p className="muted">{t.profile.note}</p>

        <Section title={t.profile.personas} note={t.profile.personasNote}>
          <ul className="personas">
            {p.personas.map((q) => <PersonaRow key={q.id} id={q.id} name={q.name} usual={q.usual} rooms={q.rooms} onChanged={() => api.profile().then(setP)} />)}
          </ul>
          {p.profile ? <Button small kind="quiet" onClick={() => open({ kind: "space", id: p.profile! })}>{t.profile.visit}</Button> : null}
        </Section>

        <Section title={t.profile.receipts} note={t.profile.receiptsNote}>
          {p.receipts.length === 0 ? <p className="muted small">{t.profile.noReceipts}</p> : null}
          <ul className="receipts">
            {p.receipts.map((b, i) => (
              <li key={i} className="receipt">
                <span className="receipt-name">✓ {b.title}</span>
                <span className="muted small">{t.profile.earnedBy} <strong>{b.to}</strong> · {t.profile.issuedBy} <button className="space-chip" onClick={() => open({ kind: "space", id: b.space.id })}>{b.space.title}</button> · {b.holds ? `${t.profile.holds} ${b.issued_by}` : t.profile.doesntHold} · {time(b.at)}</span>
              </li>
            ))}
          </ul>
        </Section>

        <Section title={t.profile.shows}>
          {p.shows.length === 0 ? <p className="muted small">{t.profile.noShows}</p> : null}
          {p.shows.map((m) => (
            <article key={m.id} className="feed-card move-show">
              <header className="move-head"><span className="muted small">{time(m.at)}</span></header>
              <h3 className="move-title">{m.title}</h3>
              {m.body ? <div className="move-body"><Markdown text={m.body} /></div> : null}
            </article>
          ))}
        </Section>

        <Section title={t.profile.agents}>
          <ul className="space-list">
            {p.agents.map((a) => (
              <li key={a.name}>
                <button className="space-row" onClick={() => open({ kind: "agent", name: a.name })}>
                  <span className="space-row-title">{a.name}</span>
                  <Chip>{kindTitle(a.image_name)}</Chip>
                  <span className="muted small">{a.active ? t.status.working : a.last_active ? `${t.status.idle} · ${ago(a.last_active)}` : t.status.never}</span>
                </button>
              </li>
            ))}
          </ul>
        </Section>

        <Section title={t.profile.offers} note={t.profile.offersNote}>
          <ul className="space-list">
            {p.machines.map((m, i) => (
              <li key={i}>
                <button className="space-row" onClick={() => open({ kind: "machines" })}>
                  <span className="space-row-title mono">{providerName(m.identity)}</span>
                  <span className="muted small">{providerWay(m.identity)}</span>
                  {m.volumes.map((v) => <Chip key={v.name}>{v.name}</Chip>)}
                </button>
              </li>
            ))}
          </ul>
          <p className="muted small">{t.profile.storage}: {p.volumes.length} {t.profile.volumesLine} · {bytes(p.volumes.reduce((n, v) => n + v.bytes, 0))}</p>
        </Section>
      </div>
    </div>
  );
}

function PersonaRow(props: { id: string; name: string; usual: boolean; rooms: string[]; onChanged: () => void }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(props.name);
  return (
    <li className="persona">
      {editing ? (
        <>
          <input value={name} onChange={(e) => setName(e.target.value)} />
          <Button small kind="primary" onClick={() => api.personaRename(props.id, name).then(() => { setEditing(false); props.onChanged(); })} disabled={!name.trim()}>{t.profile.save}</Button>
        </>
      ) : (
        <>
          <strong>{props.name}</strong>
          {props.usual ? <Chip>{t.profile.usualTag}</Chip> : null}
          <button className="link small" onClick={() => setEditing(true)}>{t.profile.rename}</button>
        </>
      )}
      <span className="muted small">{props.rooms.length ? `${t.profile.inRooms} ${props.rooms.join(", ")}` : t.profile.noRooms}</span>
    </li>
  );
}

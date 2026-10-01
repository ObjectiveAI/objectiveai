import { useEffect, useState } from "react";
import type { MoveView } from "../bindings/MoveView";
import type { ProfileView } from "../bindings/ProfileView";
import { Markdown } from "../components/Markdown";
import { Button, Chip, Empty, Row, Section, SectionHead } from "../components/ui";
import { useShared } from "../lib/context";
import { ago, bytes, kindTitle, providerName, providerWay, time, spaceTitle } from "../lib/format";
import { api } from "../lib/ipc";
import { t } from "../strings";

export function Profile() {
  const { open } = useShared();
  const [p, setP] = useState<ProfileView | null>(null);
  const [room, setRoom] = useState<MoveView[]>([]);
  const [pinned, setPinned] = useState<string | null>(null);
  useEffect(() => {
    api.profile().then(async (view) => {
      setP(view);
      if (view.profile) {
        const feed = await api.spaceFeed(view.profile);
        if (feed.outcome === "feed") setRoom(feed.moves);
      }
    });
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
          {p.profile ? <Button small kind="tertiary" onClick={() => open({ kind: "space", id: p.profile! })}>{t.profile.visit}</Button> : null}
        </Section>

        {p.profile ? (
          <Section title={t.profile.visit} note={t.profile.visitNote}>
            <SectionHead small level={3} title={t.profile.hires} />
            {room.filter((m) => m.kind === "hire").length === 0 ? <p className="muted small">{t.profile.noHires}</p> : null}
            <ul className="member-list">
              {room.filter((m) => m.kind === "hire").map((m) => (
                <li key={m.id} className="member">
                  <span><strong>{m.author}</strong>: {m.title}</span>
                  <Chip tone={m.state === "delivered" ? "ok" : "plain"}>{t.spaces.states[m.state] ?? m.state}</Chip>
                </li>
              ))}
            </ul>
            <SectionHead small level={3} title={t.profile.notes} />
            {room.filter((m) => m.kind === "note").length === 0 ? <p className="muted small">{t.profile.noNotes}</p> : null}
            <ul className="member-list">
              {room.filter((m) => m.kind === "note").map((m) => (
                <li key={m.id} className="member">
                  <span><strong>{m.author}</strong>: “{m.body}”</span>
                  <span className="muted small">{time(m.at)}</span>
                </li>
              ))}
            </ul>
          </Section>
        ) : null}

        <Section title={t.profile.receipts} note={t.profile.receiptsNote}>
          {p.receipts.length === 0 ? <p className="muted small">{t.profile.noReceipts}</p> : null}
          <ul className="receipts">
            {p.receipts.map((b, i) => (
              <li key={i} className="receipt">
                <span className="receipt-name">✓ {b.title}</span>
                {p.profile && b.holds && b.earned_as_usual ? (
                  <Button small kind="tertiary" onClick={() => api.spaceCall(p.profile!, "pin_receipt", { statement: b.statement }).then((out) => setPinned(out.outcome === "ok" ? b.title : out.message))}>
                    {pinned === b.title ? t.profile.pinned : t.profile.pin}
                  </Button>
                ) : null}
                <span className="muted small">
                  {t.profile.earnedBy} <strong>{b.to}</strong> · {t.profile.issuedBy}{" "}
                  {b.space ? <Chip onClick={() => open({ kind: "space", id: b.space!.id })}>{b.room_title}</Chip> : <strong>{b.room_title}</strong>} ·{" "}
                  {b.holds ? `${t.profile.holds} ${b.issued_by}, ${b.known ? t.profile.someoneYouKnow : t.profile.someoneNew}` : t.profile.doesntHold} · {time(b.at)}
                </span>
                {b.holds && !b.earned_as_usual ? <span className="muted small">{t.profile.earnedAsFresh} {b.earned_as}{t.profile.earnedAsFreshNote}</span> : null}
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
                <Row className="space-row" onClick={() => open({ kind: "agent", name: a.name })}>
                  <span className="space-row-title">{a.name}</span>
                  <Chip>{kindTitle(a.image_name)}</Chip>
                  <span className="muted small">{a.active ? t.status.working : a.last_active ? `${t.status.idle} · ${ago(a.last_active)}` : t.status.never}</span>
                </Row>
              </li>
            ))}
          </ul>
        </Section>

        <Section title={t.profile.offers} note={t.profile.offersNote}>
          <ul className="space-list">
            {p.machines.map((m, i) => (
              <li key={i}>
                <Row className="space-row" onClick={() => open({ kind: "machines" })}>
                  <span className="space-row-title mono">{providerName(m.identity)}</span>
                  <span className="muted small">{providerWay(m.identity)}</span>
                  {m.volumes.map((v) => <Chip key={v.name}>{v.name}</Chip>)}
                </Row>
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
  const { spaces } = useShared();
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
          <Button small kind="tertiary" onClick={() => setEditing(true)}>{t.profile.rename}</Button>
        </>
      )}
      <span className="muted small">{props.rooms.length ? `${t.profile.inRooms} ${props.rooms.map((id) => { const r = spaces.find((x) => x.id === id); return r ? spaceTitle(r) : id; }).join(", ")}` : t.profile.noRooms}</span>
    </li>
  );
}

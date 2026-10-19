import { useState } from "react";
import { KnockCard } from "../components/Knock";
import { Button, Chip, Dot, Field, Row, Section, Segmented } from "../components/ui";
import { useShared } from "../lib/context";
import { providerName, spaceTitle } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";

export function Spaces() {
  const { spaces, refreshSpaces, knocks, answerKnock, open } = useShared();
  const [host, setHost] = useState({ title: "", kind: "home", charter: "", open_door: false });
  const [hosting, setHosting] = useState<string | null>(null);
  const [invite, setInvite] = useState("");

  const doHost = async () => {
    setHosting(null);
    try {
      const out = await api.spaceHost({ ...host, title: host.title.trim() });
      if (out.outcome === "hosted") {
        await refreshSpaces();
        setHost({ title: "", kind: "home", charter: "", open_door: false });
        open({ kind: "space", id: out.id });
      } else setHosting(out.message);
    } catch (e) {
      setHosting(errorText(e));
    }
  };

  const row = (s: (typeof spaces)[number]) => (
    <li key={s.id}>
      <Row className="space-row" onClick={() => open({ kind: "space", id: s.id })}>
        <Dot state={s.online ? "idle" : "never"} />
        <span className="space-row-title">{spaceTitle(s)}</span>
        <Chip>{t.spaces.kinds[s.kind] ?? s.kind}</Chip>
        <span className="muted small">{s.mine ? t.spaces.youHostIt : `${t.spaces.hostedBy} ${s.host_name} · ${providerName(s.host)}`}{s.online ? "" : ` · ${t.spaces.offline}`}{s.fresh ? ` · ${t.spaces.youAreHere} ${s.you_are}` : ""}</span>
      </Row>
    </li>
  );

  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.spaces.title}</h1>
        <p className="muted">{t.spaces.note}</p>

        {knocks.length > 0 ? (
          <Section title={t.spaces.door} note={t.spaces.doorNote}>
            <ul className="knocks">
              {knocks.map((k) => (
                <li key={k.knock_id}>
                  <KnockCard knock={k} onAnswer={(yes) => answerKnock(k.knock_id, yes)} />
                </li>
              ))}
            </ul>
          </Section>
        ) : null}

        <Section title={t.spaces.youHost}>
          {spaces.filter((s) => s.mine).length === 0 ? <p className="muted small">{t.spaces.none}</p> : <ul className="space-list">{spaces.filter((s) => s.mine).map(row)}</ul>}
        </Section>
        <Section title={t.spaces.joined}>
          {spaces.filter((s) => !s.mine).length === 0 ? <p className="muted small">{t.spaces.none}</p> : <ul className="space-list">{spaces.filter((s) => !s.mine).map(row)}</ul>}
        </Section>

        <Section title={t.spaces.host} note={t.spaces.hostNote}>
          <div className="row">
            <Field label={t.spaces.hostTitle}>
              <input value={host.title} onChange={(e) => setHost({ ...host, title: e.target.value })} />
            </Field>
            <Field label={t.spaces.hostKind}>
              <Segmented value={host.kind} options={Object.entries(t.spaces.kinds).filter(([value]) => value !== "profile").map(([value, label]) => ({ value, label }))} onChange={(kind) => setHost({ ...host, kind })} />
            </Field>
          </div>
          <Field label={t.spaces.hostCharter} wide>
            <textarea rows={4} value={host.charter} placeholder={t.spaces.hostCharterPlaceholder} onChange={(e) => setHost({ ...host, charter: e.target.value })} />
          </Field>
          <label className="switch switch-line">
            <input type="checkbox" checked={host.open_door} onChange={(e) => setHost({ ...host, open_door: e.target.checked })} />
            <span>{t.spaces.hostOpenDoor}</span>
          </label>
          <p className="muted small">{t.spaces.hostOpenDoorNote}</p>
          <div className="row-actions">
            <Button kind="primary" onClick={doHost} disabled={!host.title.trim()}>{t.spaces.hostButton}</Button>
            {hosting ? <span className="bad small">{hosting}</span> : null}
          </div>
        </Section>

        <Section title={t.spaces.join} note={t.door.beforeNote}>
          <div className="row">
            <Field label={t.door.paste} size="grow">
              <input className="mono" value={invite} placeholder="diverge-invite:…" onChange={(e) => setInvite(e.target.value)} spellCheck={false} />
            </Field>
          </div>
          <div className="row-actions">
            <Button kind="primary" onClick={() => { open({ kind: "door", invite: invite.trim() }); setInvite(""); }} disabled={!invite.trim().startsWith("diverge-invite:")}>{t.door.read}</Button>
          </div>
        </Section>
      </div>
    </div>
  );
}

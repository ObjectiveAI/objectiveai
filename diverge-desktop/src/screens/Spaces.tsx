import { useState } from "react";
import { Button, Chip, Dot, Field, Section, Segmented } from "../components/ui";
import { useShared } from "../lib/context";
import { ago, providerName } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";

export function Spaces() {
  const { spaces, refreshSpaces, knocks, answerKnock, open } = useShared();
  const [host, setHost] = useState({ title: "", kind: "home", charter: "", invite: "" });
  const [hosting, setHosting] = useState<string | null>(null);
  const [invite, setInvite] = useState("");
  const [asName, setAsName] = useState("");
  const [joinSaid, setJoinSaid] = useState<string | null>(null);

  const doHost = async () => {
    setHosting(null);
    try {
      const out = await api.spaceHost({ ...host, title: host.title.trim(), invite: host.invite.trim() || `${host.title.trim().toLowerCase().replace(/\s+/g, "-")}-key` });
      if (out.outcome === "hosted") {
        await refreshSpaces();
        setHost({ title: "", kind: "home", charter: "", invite: "" });
        open({ kind: "space", id: out.id });
      } else setHosting(out.message);
    } catch (e) {
      setHosting(errorText(e));
    }
  };

  const doJoin = async () => {
    setJoinSaid(null);
    try {
      const out = await api.spaceJoin(invite.trim(), asName.trim());
      if (out.outcome === "joined") {
        setJoinSaid(t.spaces.joinedOk);
        setInvite("");
        await refreshSpaces();
        open({ kind: "space", id: out.id });
      } else setJoinSaid(out.outcome === "denied" ? t.spaces.joinDenied : out.outcome === "missing" ? t.spaces.joinMissing : out.message);
    } catch (e) {
      setJoinSaid(errorText(e));
    }
  };

  const row = (s: (typeof spaces)[number]) => (
    <li key={s.id}>
      <button className="space-row" onClick={() => open({ kind: "space", id: s.id })}>
        <Dot state={s.online ? "idle" : "never"} />
        <span className="space-row-title">{s.title}</span>
        <Chip>{t.spaces.kinds[s.kind] ?? s.kind}</Chip>
        <span className="muted small">{s.mine ? t.spaces.youHostIt : `${t.spaces.hostedBy} ${providerName(s.host)}`}{s.online ? "" : ` · ${t.spaces.offline}`}</span>
      </button>
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
                <li key={k.knock_id} className="knock">
                  <div className="knock-main">
                    <div><strong className="mono">{k.address}</strong> <span className="muted">{t.spaces.presented}</span> <span className="mono selectable">{k.authorization}</span></div>
                    <div className="muted small">{k.space_title} · {ago(k.at)}</div>
                  </div>
                  <Button small kind="primary" onClick={() => answerKnock(k.knock_id, true)}>{t.spaces.letIn}</Button>
                  <Button small kind="quiet" onClick={() => answerKnock(k.knock_id, false)}>{t.spaces.notNow}</Button>
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
              <Segmented value={host.kind} options={Object.entries(t.spaces.kinds).map(([value, label]) => ({ value, label }))} onChange={(kind) => setHost({ ...host, kind })} />
            </Field>
          </div>
          <Field label={t.spaces.hostCharter} wide>
            <textarea rows={4} value={host.charter} placeholder={t.spaces.hostCharterPlaceholder} onChange={(e) => setHost({ ...host, charter: e.target.value })} />
          </Field>
          <div className="row">
            <Field label={t.spaces.hostInvite} hint={t.spaces.hostInviteNote}>
              <input className="mono" value={host.invite} onChange={(e) => setHost({ ...host, invite: e.target.value })} spellCheck={false} />
            </Field>
          </div>
          <div className="row-actions">
            <Button kind="primary" onClick={doHost} disabled={!host.title.trim()}>{t.spaces.hostButton}</Button>
            {hosting ? <span className="bad small">{hosting}</span> : null}
          </div>
        </Section>

        <Section title={t.spaces.join}>
          <div className="row">
            <Field label={t.spaces.invite} size="grow">
              <input className="mono" value={invite} placeholder={t.spaces.joinPlaceholder} onChange={(e) => setInvite(e.target.value)} spellCheck={false} />
            </Field>
            <Field label={t.spaces.joinAs}>
              <input value={asName} onChange={(e) => setAsName(e.target.value)} />
            </Field>
          </div>
          <div className="row-actions">
            <Button kind="primary" onClick={doJoin} disabled={!invite.trim().startsWith("diverge://")}>{t.spaces.joinButton}</Button>
            {joinSaid ? <span className={joinSaid === t.spaces.joinedOk ? "ok small" : "warn small"}>{joinSaid}</span> : null}
          </div>
        </Section>
      </div>
    </div>
  );
}

import { useEffect, useState } from "react";
import type { DoorView } from "../bindings/DoorView";
import { Markdown } from "../components/Markdown";
import { ProfileCard } from "../components/ProfileCard";
import { Button, Chip, Empty, Field, Section } from "../components/ui";
import { useShared } from "../lib/context";
import { providerName, time } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";

type State = { kind: "reading" } | { kind: "waiting" } | { kind: "in"; id: string; rulesMatch: boolean } | { kind: "refused"; message: string };

/** An invite, read before knocking: nothing is sent until you knock. */
export function DoorPage(props: { invite: string; tabKey: string }) {
  const { open, close, refreshSpaces } = useShared();
  const [door, setDoor] = useState<DoorView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [fresh, setFresh] = useState(false);
  const [freshName, setFreshName] = useState("");
  const [note, setNote] = useState("");
  const [listed, setListed] = useState(true);
  const [vouch, setVouch] = useState("");
  const [state, setState] = useState<State>({ kind: "reading" });

  useEffect(() => {
    api.spaceDoor(props.invite).then(setDoor, (e) => setProblem(errorText(e)));
  }, [props.invite]);

  if (problem) return <Empty title={t.door.notAnInvite} body={problem} />;
  if (!door) return <Empty title={t.door.before} />;

  const knock = async () => {
    setState({ kind: "waiting" });
    try {
      const out = await api.spaceJoin(props.invite, fresh ? { as: "fresh", name: freshName.trim() } : { as: "usual" }, note.trim(), listed, vouch.trim() || null);
      if (out.outcome === "joined") {
        await refreshSpaces();
        setState({ kind: "in", id: out.id, rulesMatch: out.rules_match });
      } else setState({ kind: "refused", message: out.outcome === "denied" ? t.door.denied : out.outcome === "missing" ? t.door.missing : out.message });
    } catch (e) {
      setState({ kind: "refused", message: errorText(e) });
    }
  };
  const ready = (!fresh || freshName.trim() !== "") && (door.invited || note.trim() !== "");
  const as = door.sends.usual;
  /** What one field of the knock will say, from what's on this page now. */
  const sent = (field: string): string => {
    switch (field) {
      case "name": return fresh ? freshName.trim() || t.door.sendsFreshName : as.name;
      case "key": return fresh ? t.door.sendsFreshKey : as.key_mark;
      case "account": return fresh ? t.door.sendsFreshAccount : `${t.door.sendsMade} ${time(as.account_made)}, ${t.door.sendsListNumber} ${as.list_number}, ${as.devices} ${t.door.sendsDevices}`;
      case "note": return note.trim() ? `“${note.trim()}”` : t.door.sendsNoNote;
      case "listed": return listed ? t.door.sendsYes : t.door.sendsNo;
      case "invite": return door.sends.invite_mark ? t.door.sendsYes : t.door.sendsNoInvite;
      case "vouch": return vouch.trim() ? t.door.sendsYes : t.door.sendsNoVouch;
      case "room": return door.sends.room;
      default: return "";
    }
  };

  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.door.before}</h1>
        <p className="muted">{t.door.beforeNote}</p>
        <div className="door-head">
          <strong>{door.title}</strong>
          <Chip>{t.spaces.kinds[door.kind] ?? door.kind}</Chip>
          <span className="muted small">{t.door.hostedBy} {door.host_name} · {providerName(door.host)}</span>
        </div>
        {door.already_in ? <div className="banner banner-quiet">{t.door.alreadyIn}</div> : null}
        {door.card ? (
          <Section title={t.door.cardTitle} note={t.door.cardNote}>
            <ProfileCard name={door.host_name} picture={door.card.picture} about={door.card.about} links={door.card.links} />
          </Section>
        ) : null}

        <Section title={t.door.rules} note={`${t.door.rulesSetBy} ${door.host_name}. ${t.door.rulesKeep}`}>
          {door.charter.trim() ? <Markdown text={door.charter} /> : <p className="muted small">{t.door.noCharter}</p>}
        </Section>
        <Section title={t.door.verbsTitle} note={t.door.verbsNote}>
          <ul className="door-verbs">
            {door.verbs.map((v) => (
              <li key={v.name}><strong>{t.spaces.verbNames[v.name] ?? v.name.replace(/_/g, " ")}</strong>: {v.does}</li>
            ))}
          </ul>
        </Section>
        {door.already_in || state.kind === "in" ? null : (
          <Section title={t.door.sendsTitle} note={t.door.sendsNote}>
            <ul className="door-verbs">
              {door.sends.fields.map((field) => (
                <li key={field}><strong>{t.door.sends[field] ?? field}</strong>{sent(field) ? `: ${sent(field)}` : null}</li>
              ))}
            </ul>
            {fresh ? null : <p className="muted small">{t.door.sendsLinks}</p>}
          </Section>
        )}

        {state.kind === "in" ? (
          <Section title={t.door.in}>
            {state.rulesMatch ? null : <div className="banner banner-warn">{t.door.rulesDiffer}</div>}
            <Button kind="primary" onClick={() => { open({ kind: "space", id: state.id }); close(props.tabKey); }}>{t.door.open}</Button>
          </Section>
        ) : door.already_in ? null : (
          <Section title={t.door.appearAs}>
            <label className="radio-line">
              <input type="radio" checked={!fresh} onChange={() => setFresh(false)} /> {door.usual_name}
            </label>
            <label className="radio-line">
              <input type="radio" checked={fresh} onChange={() => setFresh(true)} /> {t.door.fresh}
            </label>
            {fresh ? (
              <>
                <Field label={t.door.fresh}>
                  <input value={freshName} placeholder={t.door.freshPlaceholder} onChange={(e) => setFreshName(e.target.value)} />
                </Field>
                <p className="muted small">{t.door.freshNote}</p>
              </>
            ) : null}
            <Field label={t.door.noteLabel} wide needed={!door.invited && !note.trim()}>
              <textarea rows={2} value={note} placeholder={t.door.notePlaceholder} onChange={(e) => setNote(e.target.value)} />
            </Field>
            {door.invited ? null : <p className="muted small">{t.door.openDoor}</p>}
            <Field label={t.door.vouchLabel} wide>
              <input className="mono" value={vouch} placeholder={t.door.vouchPlaceholder} onChange={(e) => setVouch(e.target.value)} spellCheck={false} />
            </Field>
            <label className="switch switch-line">
              <input type="checkbox" checked={listed} onChange={(e) => setListed(e.target.checked)} />
              <span>{t.door.listed}</span>
            </label>
            <p className="muted small">{t.door.listedNote}</p>
            <div className="row-actions">
              <Button kind="primary" onClick={knock} disabled={!ready || state.kind === "waiting"}>{state.kind === "waiting" ? t.door.waiting : t.door.knock}</Button>
              {state.kind === "refused" ? <span className="warn small">{state.message}</span> : null}
            </div>
          </Section>
        )}
      </div>
    </div>
  );
}

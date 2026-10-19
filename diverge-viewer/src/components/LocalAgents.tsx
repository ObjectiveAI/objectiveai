import { useEffect, useState } from "react";
import type { DoorStatusView } from "../bindings/DoorStatusView";
import type { LocalAgentView } from "../bindings/LocalAgentView";
import { ago } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";
import { Button, Escapes, Field } from "./ui";

/** Agents you run yourself on this Mac: add one, copy the line that connects it, give it a new key, remove it. */
export function LocalAgents() {
  const w = t.machines.local;
  const [agents, setAgents] = useState<LocalAgentView[]>([]);
  const [status, setStatus] = useState<DoorStatusView | null>(null);
  const [name, setName] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [said, setSaid] = useState<Record<string, string>>({});
  const [confirm, setConfirm] = useState<string | null>(null);

  const load = async () => {
    setAgents(await api.localAgents());
    setStatus(await api.doorStatus());
  };
  useEffect(() => {
    load().catch((e) => setProblem(errorText(e)));
  }, []);

  const act = async (f: () => Promise<unknown>) => {
    setProblem(null);
    try {
      await f();
      await load();
    } catch (e) {
      setProblem(errorText(e));
    }
  };

  const copy = async (a: LocalAgentView) => {
    try {
      await navigator.clipboard.writeText(a.connect_line ?? "");
      setSaid({ ...said, [a.id]: w.copied });
    } catch {
      setSaid({ ...said, [a.id]: w.copyFailed });
    }
  };

  return (
    <section className="add-machine">
      <h2>{w.title}</h2>
      <p className="muted small">{w.note}</p>
      {status ? (
        <p className="muted small">
          {w.door[status.state]}
          {status.port !== null && (status.state === "listening" || status.state === "port_taken") ? <span className="mono"> {status.port}</span> : null}
        </p>
      ) : null}
      {agents.length ? (
        <ul className="machines">
          {agents.map((a) => (
            <li key={a.id} className="machine">
              <div className="machine-main">
                <div>{a.name} <span className="muted small mono">{a.slot}</span> <span className="muted small">· {t.machines.added} {ago(a.added)}</span></div>
                {a.connect_line ? (
                  <>
                    <div className="muted small">{w.connect}</div>
                    <div className="mono small selectable">{a.connect_line}</div>
                    <div className="muted small">{w.connectNote}</div>
                    {said[a.id] ? <div className="muted small">{said[a.id]}</div> : null}
                  </>
                ) : null}
              </div>
              <div className="machine-actions">
                <Button small kind="secondary" onClick={() => copy(a)} disabled={!a.connect_line}>{w.copy}</Button>
                <Button small kind="tertiary" title={w.newKeyNote} onClick={() => act(() => api.localAgentNewKey(a.id))}>{w.newKey}</Button>
                {confirm === a.id ? (
                  <Escapes className="confirm-pair" onEscape={() => setConfirm(null)}>
                    <Button small kind="danger" onClick={() => act(async () => { await api.localAgentRemove(a.id); setConfirm(null); })}>{w.confirm}</Button>
                    <Button small kind="tertiary" onClick={() => setConfirm(null)}>{w.keep}</Button>
                  </Escapes>
                ) : (
                  <Button small kind="tertiary" onClick={() => setConfirm(a.id)}>{w.remove}</Button>
                )}
              </div>
            </li>
          ))}
        </ul>
      ) : (
        <p className="muted small">{w.none}</p>
      )}
      <div className="row">
        <Field label={w.name}>
          <input value={name} placeholder={w.namePlaceholder} onChange={(e) => setName(e.target.value)} />
        </Field>
      </div>
      <div className="row-actions">
        <Button kind="primary" disabled={!name.trim()} onClick={() => act(async () => { await api.localAgentAdd(name); setName(""); })}>{w.add}</Button>
      </div>
      {problem ? <div className="banner banner-bad">{problem}</div> : null}
    </section>
  );
}

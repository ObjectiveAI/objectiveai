import { useEffect, useState } from "react";
import type { MachineView } from "../bindings/MachineView";
import { LocalAgents } from "../components/LocalAgents";
import { Button, Chip, Field, Icon, Segmented } from "../components/ui";
import { useShared } from "../lib/context";
import { ago, providerName, providerRaw, providerWay, setMachineNames } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";

export function Machines() {
  const { agents } = useShared();
  const [machines, setMachines] = useState<MachineView[]>([]);
  const [way, setWay] = useState<"dial" | "accept">("dial");
  const [address, setAddress] = useState("");
  const [identity, setIdentity] = useState("");
  const [key, setKey] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<string | null>(null);

  const load = () => api.machines().then(setMachines);
  const rename = async (m: MachineView, name: string) => {
    setMachineNames(await api.machineRename(m.identity, name));
    load();
  };
  useEffect(() => {
    load();
  }, []);

  const add = async () => {
    setProblem(null);
    try {
      await api.machineAdd(way === "dial" ? { way: "dial", address, key } : { way: "accept", identity, key });
      setAddress("");
      setIdentity("");
      setKey("");
      load();
    } catch (e) {
      setProblem(errorText(e));
    }
  };

  const remove = async (m: MachineView) => {
    setProblem(null);
    try {
      await api.machineRemove(m.identity);
      setConfirm(null);
      load();
    } catch (e) {
      setProblem(errorText(e));
      setConfirm(null);
    }
  };

  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.machines.title}</h1>
        <p className="muted">{t.machines.note}</p>

        <ul className="machines">
          {machines.map((m) => {
            const id = providerName(m.identity);
            const here = agents.filter((a) => a.active && a.provider && providerName(a.provider) === id);
            return (
              <li key={id} className="machine">
                <div className="machine-icon"><Icon name="machine" /></div>
                <div className="machine-main">
                  <input className="machine-name-input" defaultValue={m.name ?? ""} placeholder={t.machines.namePlaceholder} aria-label={t.machines.name} onBlur={(e) => { if ((e.target.value.trim() || "") !== (m.name ?? "")) rename(m, e.target.value); }} onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }} />
                  <div className="muted small">{t.machines.daemonName} <span className="mono selectable">{providerRaw(m.identity)}</span> · {providerWay(m.identity)} · {t.machines.added} {ago(m.added)}</div>
                  <div className="machine-storage">
                    <span className="muted small">{t.machines.storage}:</span>
                    {m.volumes.length ? m.volumes.map((v) => <Chip key={v.name} title={t.storage.modeNotes[v.mode]}>{v.name} · {t.storage.modes[v.mode]}</Chip>) : <span className="muted small">{m.volumes_problem ?? t.machines.noStorage}</span>}
                  </div>
                  {here.length ? <div className="small ok">{here.map((a) => a.name).join(", ")} — {t.status.working.toLowerCase()}</div> : null}
                </div>
                <div className="machine-actions">
                  {confirm === id ? (
                    <>
                      <Button small kind="danger" onClick={() => remove(m)}>{t.machines.confirm}</Button>
                      <Button small kind="tertiary" onClick={() => setConfirm(null)}>{t.machines.keep}</Button>
                    </>
                  ) : (
                    <Button small kind="tertiary" onClick={() => setConfirm(id)}>{t.machines.remove}</Button>
                  )}
                </div>
              </li>
            );
          })}
        </ul>
        {problem ? <div className="banner banner-bad">{problem}</div> : null}

        <section className="add-machine">
          <h2>{t.machines.add}</h2>
          <Segmented value={way} options={[{ value: "dial", label: t.machines.dial }, { value: "accept", label: t.machines.accept }]} onChange={setWay} />
          <div className="row">
            {way === "dial" ? (
              <Field label={t.machines.address}>
                <input className="mono" value={address} placeholder={t.machines.addressPlaceholder} onChange={(e) => setAddress(e.target.value)} spellCheck={false} />
              </Field>
            ) : (
              <Field label={t.machines.identity}>
                <input className="mono" value={identity} placeholder={t.machines.identityPlaceholder} onChange={(e) => setIdentity(e.target.value)} spellCheck={false} />
              </Field>
            )}
            <Field label={t.machines.key} hint={t.machines.keyNote}>
              <input type="password" value={key} onChange={(e) => setKey(e.target.value)} />
            </Field>
          </div>
          <div className="row-actions">
            <Button kind="primary" onClick={add} disabled={!key.trim() || !(way === "dial" ? address.trim() : identity.trim())}>{t.machines.save}</Button>
          </div>
          <p className="muted small">{t.machines.ours}</p>
        </section>

        <LocalAgents />
      </div>
    </div>
  );
}

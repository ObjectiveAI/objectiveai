import { useEffect, useMemo, useState } from "react";
import type { ImageKindView } from "../bindings/ImageKindView";
import type { MachineView } from "../bindings/MachineView";
import { LiveMounts, MountRows, emptyDraft, inputsOf, newRow, type MountsDraft, scratchMissing } from "../components/Mounts";
import { SchemaForm, initial, missing, type Schema } from "../components/SchemaForm";
import { Button, Field, Section, Segmented } from "../components/ui";
import { useShared } from "../lib/context";
import { providerName, providerWay } from "../lib/format";
import { api, errorText, ticket } from "../lib/ipc";
import { t } from "../strings";

const GB = 1024 ** 3;

export function NewAgent(props: { tabKey: string }) {
  const { open, close, refreshAgents, agents } = useShared();
  const [catalog, setCatalog] = useState<ImageKindView[]>([]);
  const [machines, setMachines] = useState<MachineView[]>([]);
  const [kind, setKind] = useState("cc");
  const [settings, setSettings] = useState<Record<string, unknown>>({});
  const [check, setCheck] = useState<{ ok: boolean; message?: string } | null>(null);
  const [memory, setMemory] = useState(4);
  const [disk, setDisk] = useState(8);
  const [where, setWhere] = useState<"anywhere" | "pinned">("anywhere");
  const [machine, setMachine] = useState(0);
  const [mounts, setMounts] = useState<MountsDraft>(emptyDraft());
  const [name, setName] = useState("");
  const [first, setFirst] = useState("");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    api.catalog().then((c) => {
      setCatalog(c);
      const cc = c.find((x) => x.key === "cc") ?? c[0];
      if (cc) setSettings(initial(cc.schema as Schema, cc.schema as Schema) as Record<string, unknown>);
    });
    api.machines().then(setMachines);
  }, []);

  const image = catalog.find((c) => c.key === kind);
  const pick = (key: string) => {
    setKind(key);
    const next = catalog.find((c) => c.key === key);
    if (next) setSettings(initial(next.schema as Schema, next.schema as Schema) as Record<string, unknown>);
  };

  // Checked with the image's own types (Ronald's source), as you type.
  useEffect(() => {
    let live = true;
    const timer = setTimeout(() => {
      api.check(kind, settings).then(
        () => live && setCheck({ ok: true }),
        (e) => live && setCheck({ ok: false, message: errorText(e) }),
      );
    }, 250);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [kind, settings]);

  const chosen = machines[machine];
  const empty = useMemo(() => (image ? missing(image.schema as Schema, settings) : []), [image, settings]);
  const taken = agents.some((a) => a.name === name.trim());
  const ready = !!image && name.trim() !== "" && !taken && !!check?.ok && empty.length === 0 && (where === "anywhere" || !!chosen);

  const create = async () => {
    if (!ready) return;
    setBusy(true);
    setProblem(null);
    try {
      const outcome = await api.agentsCreate({
        name: name.trim(),
        image_kind: kind,
        memory: Math.round(memory * GB),
        disk: Math.round(disk * GB),
        provider: where === "pinned" && chosen ? chosen.identity : null,
        ...inputsOf(mounts, machines, where === "pinned" && chosen ? chosen.identity : null),
        arguments: settings,
      });
      if (outcome.outcome === "created") {
        await refreshAgents();
        open({ kind: "agent", name: name.trim() });
        close(props.tabKey);
        if (first.trim()) api.agentsMessage(name.trim(), first.trim(), ticket()).then(refreshAgents);
      } else if (outcome.outcome === "in_use") setProblem(t.create.inUse);
      else setProblem(outcome.message);
    } catch (e) {
      setProblem(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const schema = useMemo(() => image?.schema as Schema | undefined, [image]);

  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.create.title}</h1>

        <Section step={1} title={t.create.kind}>
          <div className="kinds">
            {catalog.map((c) => (
              <button key={c.key} className={`kind${kind === c.key ? " on" : ""}`} onClick={() => pick(c.key)}>
                <span className="kind-title">{t.create.kinds[c.key]?.title ?? c.key}</span>
                <span className="kind-blurb">{t.create.kinds[c.key]?.blurb}</span>
              </button>
            ))}
          </div>
        </Section>

        <Section step={2} title={t.create.settings} aside={check?.ok ? <span className={empty.length ? "muted small" : "ok small"}>{empty.length ? `${t.create.fillIn} ${empty.join(", ")}` : "✓ " + t.create.ready}</span> : null}>
          {schema ? <SchemaForm key={kind} schema={schema} value={settings} onChange={(v) => setSettings(v as Record<string, unknown>)} /> : null}
          {check && !check.ok ? <p className="check-bad small"><strong>{t.create.notReady}</strong> {check.message}</p> : null}
        </Section>

        <Section step={3} title={t.create.limits} note={t.create.limitsNote}>
          <div className="row">
            <Field label={t.create.memory}>
              <input type="number" min={0.25} step={0.25} value={memory} onChange={(e) => setMemory(Number(e.target.value))} />
            </Field>
            <Field label={t.create.disk}>
              <input type="number" min={0.25} step={0.25} value={disk} onChange={(e) => setDisk(Number(e.target.value))} />
            </Field>
          </div>
        </Section>

        <Section step={4} title={t.create.where}>
          <Segmented value={where} options={[{ value: "anywhere", label: t.create.anywhere }, { value: "pinned", label: t.create.pinned }]} onChange={setWhere} />
          <p className="muted small">{where === "anywhere" ? t.create.anywhereNote : t.create.pinnedNote}</p>
          {where === "pinned" ? (
            <div className="stack">
              <Field label={t.create.machine}>
                <select value={machine} onChange={(e) => { setMachine(Number(e.target.value)); setMounts({ ...mounts, own: [] }); }}>
                  {machines.map((m, i) => (
                    <option key={i} value={i}>{providerName(m.identity)} — {providerWay(m.identity)}</option>
                  ))}
                </select>
              </Field>
              <MountRows rows={mounts.own} onChange={(own) => setMounts({ ...mounts, own })} machines={machines} fixed={chosen} inPlaceholder="(all of it)" />
              {chosen && chosen.volumes.length > 0 ? (
                <Button small kind="quiet" onClick={() => setMounts({ ...mounts, own: [...mounts.own, newRow(machines, chosen)] })}>+ {t.create.addStorage}</Button>
              ) : null}
            </div>
          ) : null}
        </Section>

        <Section step={5} title={t.create.share} note={t.create.shareNote}>
          <LiveMounts draft={mounts} onChange={setMounts} machines={machines} />
        </Section>

        <Section step={6} title={`${t.create.name} & ${t.create.first.toLowerCase()}`}>
          <Field label={t.create.name} needed={!name.trim()} hint={taken ? t.create.inUse : undefined}>
            <input value={name} placeholder={t.create.namePlaceholder} onChange={(e) => setName(e.target.value)} spellCheck={false} />
          </Field>
          <Field label={t.create.first} wide>
            <textarea rows={3} value={first} placeholder={t.create.firstPlaceholder} onChange={(e) => setFirst(e.target.value)} />
          </Field>
        </Section>

        <div className="page-foot">
          {problem ? <span className="bad">{problem}</span> : null}
          <Button kind="primary" onClick={create} disabled={!ready || busy || scratchMissing(mounts, machines) > 0}>{busy ? t.create.creating : t.create.create}</Button>
        </div>
      </div>
    </div>
  );
}

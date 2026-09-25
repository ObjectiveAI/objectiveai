import { useEffect, useMemo, useState } from "react";
import type { ImageKindView } from "../bindings/ImageKindView";
import type { MachineView } from "../bindings/MachineView";
import { SchemaForm, initial, missing, type Schema } from "../components/SchemaForm";
import { Button, Field, Section, Segmented } from "../components/ui";
import { useShared } from "../lib/context";
import { providerName, providerWay } from "../lib/format";
import { api, errorText, ticket } from "../lib/ipc";
import { t } from "../strings";

type Mount = { from: string; to: string };
type Storage = { volume: string; inVolume: string; to: string };

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
  const [storage, setStorage] = useState<Storage[]>([]);
  const [folders, setFolders] = useState<Mount[]>([]);
  const [files, setFiles] = useState<Mount[]>([]);
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
        volume_mounts: where === "pinned" ? storage.filter((s) => s.volume).map((s) => ({ volume_name: s.volume, volume_relative_path: s.inVolume, container_path: s.to })) : [],
        fuse_directory_mounts: folders.filter((m) => m.from).map((m) => ({ daemon_path: m.from, container_path: m.to })),
        fuse_file_mounts: files.filter((m) => m.from).map((m) => ({ daemon_path: m.from, container_path: m.to })),
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

  const mountRows = (rows: Mount[], set: (m: Mount[]) => void, placeholder: string) =>
    rows.map((m, i) => (
      <div className="row" key={i}>
        <Field label={t.create.here}>
          <input className="mono" value={m.from} placeholder={placeholder} onChange={(e) => set(rows.map((x, j) => (j === i ? { ...x, from: e.target.value } : x)))} spellCheck={false} />
        </Field>
        <Field label={t.create.inAgent}>
          <input className="mono" value={m.to} placeholder="work/notes" onChange={(e) => set(rows.map((x, j) => (j === i ? { ...x, to: e.target.value } : x)))} spellCheck={false} />
        </Field>
        <Button small kind="quiet" onClick={() => set(rows.filter((_, j) => j !== i))}>{t.create.remove}</Button>
      </div>
    ));

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
                <span className="kind-image mono">{c.image_name}</span>
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
                <select value={machine} onChange={(e) => { setMachine(Number(e.target.value)); setStorage([]); }}>
                  {machines.map((m, i) => (
                    <option key={i} value={i}>{providerName(m.identity)} — {providerWay(m.identity)}</option>
                  ))}
                </select>
              </Field>
              {storage.map((s, i) => (
                <div className="row" key={i}>
                  <Field label={t.create.volume}>
                    <select value={s.volume} onChange={(e) => setStorage(storage.map((x, j) => (j === i ? { ...x, volume: e.target.value } : x)))}>
                      <option value="">—</option>
                      {chosen?.volumes.map((v) => <option key={v} value={v}>{v}</option>)}
                    </select>
                  </Field>
                  <Field label={t.create.inVolume}>
                    <input className="mono" value={s.inVolume} placeholder="(all of it)" onChange={(e) => setStorage(storage.map((x, j) => (j === i ? { ...x, inVolume: e.target.value } : x)))} spellCheck={false} />
                  </Field>
                  <Field label={t.create.inAgent}>
                    <input className="mono" value={s.to} placeholder="workspace" onChange={(e) => setStorage(storage.map((x, j) => (j === i ? { ...x, to: e.target.value } : x)))} spellCheck={false} />
                  </Field>
                  <Button small kind="quiet" onClick={() => setStorage(storage.filter((_, j) => j !== i))}>{t.create.remove}</Button>
                </div>
              ))}
              {chosen && chosen.volumes.length > 0 ? (
                <Button small kind="quiet" onClick={() => setStorage([...storage, { volume: chosen.volumes[0], inVolume: "", to: chosen.volumes[0] }])}>+ {t.create.addStorage}</Button>
              ) : null}
            </div>
          ) : null}
        </Section>

        <Section step={5} title={t.create.share} note={t.create.shareNote}>
          {mountRows(folders, setFolders, "/projects/site")}
          {mountRows(files, setFiles, "/notes/ideas.md")}
          <div className="row-actions">
            <Button small kind="quiet" onClick={() => setFolders([...folders, { from: "", to: "" }])}>+ {t.create.addShare}</Button>
            <Button small kind="quiet" onClick={() => setFiles([...files, { from: "", to: "" }])}>+ {t.create.addFileShare}</Button>
          </div>
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
          <Button kind="primary" onClick={create} disabled={!ready || busy}>{busy ? t.create.creating : t.create.create}</Button>
        </div>
      </div>
    </div>
  );
}

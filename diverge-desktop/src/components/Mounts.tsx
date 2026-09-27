import { useEffect, useState } from "react";
import type { FuseMountInput } from "../bindings/FuseMountInput";
import type { MachineView } from "../bindings/MachineView";
import type { MountView } from "../bindings/MountView";
import type { MountsView } from "../bindings/MountsView";
import type { ProviderView } from "../bindings/ProviderView";
import type { VolumeMountInput } from "../bindings/VolumeMountInput";
import { identityKey, providerName } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";
import { Button, Field } from "./ui";

/** One mount as a form holds it. `machine` is an identity key; a pinned machine's own rows ignore it. */
export type MountRow = { machine: string; volume: string; inVolume: string; to: string; scratch?: string };

/** Scratch space a fresh-each-run volume can have when served live: the person picks; nothing is picked for them. */
export const SCRATCH_SIZES = [1, 4, 16].map((gb) => gb * 1024 ** 3);
/** The daemon's three lists: the pinned machine's own volumes, live folders, live files. */
export type MountsDraft = { own: MountRow[]; folders: MountRow[]; files: MountRow[] };

export const emptyDraft = (): MountsDraft => ({ own: [], folders: [], files: [] });

const slash = (path: string) => path.trim().replace(/^\/+/, "");

export function draftOf(view: MountsView): MountsDraft {
  const row = (m: MountView): MountRow => ({ machine: m.provider ? identityKey(m.provider) : "", volume: m.volume_name, inVolume: m.volume_relative_path, to: m.container_path, scratch: m.overlay_disk ? String(m.overlay_disk) : "" });
  return { own: view.volume_mounts.map(row), folders: view.fuse_directory_mounts.map(row), files: view.fuse_file_mounts.map(row) };
}

/** Into the daemon's lists. Each mount states the mode its volume has, from the machine's own listing. */
export function inputsOf(draft: MountsDraft, machines: MachineView[], pinned: ProviderView | null) {
  const byKey = new Map(machines.map((m) => [identityKey(m.identity), m]));
  const mode = (m: MachineView | undefined, volume: string) => m?.volumes.find((v) => v.name === volume)?.mode ?? "persistent";
  const home = pinned ? byKey.get(identityKey(pinned)) : undefined;
  const volume_mounts: VolumeMountInput[] = pinned
    ? draft.own.filter((r) => r.volume).map((r) => ({ volume_name: r.volume, volume_relative_path: slash(r.inVolume), volume_mode: mode(home, r.volume), container_path: slash(r.to) }))
    : [];
  const live = (rows: MountRow[]): FuseMountInput[] =>
    rows
      .filter((r) => r.volume && byKey.has(r.machine))
      .map((r) => {
        const m = byKey.get(r.machine)!;
        const volumeMode = mode(m, r.volume);
        // A volume that starts fresh each run is served on a layer of its own; the wire wants its size, and the person chose it.
        return { provider: m.identity, volume_name: r.volume, volume_relative_path: slash(r.inVolume), volume_mode: volumeMode, overlay_disk: volumeMode === "ephemeral" && r.scratch ? Number(r.scratch) : null, container_path: slash(r.to) };
      });
  return { volume_mounts, fuse_directory_mounts: live(draft.folders), fuse_file_mounts: live(draft.files) };
}

/** Live mounts of a fresh-each-run volume still waiting for their scratch space. */
export function scratchMissing(draft: MountsDraft, machines: MachineView[]): number {
  const byKey = new Map(machines.map((m) => [identityKey(m.identity), m]));
  const ephemeral = (r: MountRow) => byKey.get(r.machine)?.volumes.find((v) => v.name === r.volume)?.mode === "ephemeral";
  return [...draft.folders, ...draft.files].filter((r) => r.volume && ephemeral(r) && !r.scratch).length;
}

/** A new row: the given machine, or the first with a volume, and its first volume. */
export function newRow(machines: MachineView[], fixed?: MachineView): MountRow {
  const m = fixed ?? machines.find((x) => x.volumes.length > 0) ?? machines[0];
  const v = m?.volumes[0]?.name ?? "";
  return { machine: m ? identityKey(m.identity) : "", volume: v, inVolume: "", to: v };
}

/** The rows of one kind of mount. `fixed` is the pinned machine: its own rows choose only a volume. */
export function MountRows(props: { rows: MountRow[]; onChange: (rows: MountRow[]) => void; machines: MachineView[]; fixed?: MachineView; inPlaceholder: string; live?: boolean }) {
  const { rows, onChange, machines, fixed } = props;
  const set = (i: number, patch: Partial<MountRow>) => onChange(rows.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const machineOf = (r: MountRow) => fixed ?? machines.find((m) => identityKey(m.identity) === r.machine);
  return (
    <>
      {rows.map((r, i) => {
        const m = machineOf(r);
        const known = m?.volumes.some((v) => v.name === r.volume);
        return (
          <div className="row" key={i}>
            {fixed ? null : (
              <Field label={t.create.from}>
                <select
                  value={r.machine}
                  onChange={(e) => {
                    const next = machines.find((x) => identityKey(x.identity) === e.target.value);
                    const volume = next?.volumes[0]?.name ?? "";
                    set(i, { machine: e.target.value, volume, to: !r.to || r.to === r.volume ? volume : r.to, scratch: "" });
                  }}
                >
                  {machines.map((x) => (
                    <option key={identityKey(x.identity)} value={identityKey(x.identity)}>{providerName(x.identity)}</option>
                  ))}
                </select>
              </Field>
            )}
            <Field label={t.create.here}>
              <select value={r.volume} onChange={(e) => set(i, { volume: e.target.value, to: !r.to || r.to === r.volume ? e.target.value : r.to, scratch: "" })}>
                {r.volume && !known ? <option value={r.volume}>{r.volume}</option> : null}
                {m?.volumes.map((v) => (
                  <option key={v.name} value={v.name}>{v.name} · {t.storage.modes[v.mode]}</option>
                ))}
              </select>
            </Field>
            <Field label={t.create.inHere}>
              <input className="mono" value={r.inVolume} placeholder={props.inPlaceholder} onChange={(e) => set(i, { inVolume: e.target.value })} spellCheck={false} />
            </Field>
            <Field label={t.create.inAgent}>
              <input className="mono" value={r.to} placeholder="work" onChange={(e) => set(i, { to: e.target.value })} spellCheck={false} />
            </Field>
            {props.live && m?.volumes.find((v) => v.name === r.volume)?.mode === "ephemeral" ? (
              <Field label={t.create.scratch}>
                <select value={r.scratch ?? ""} onChange={(e) => set(i, { scratch: e.target.value })} className={r.scratch ? "" : "invalid"}>
                  <option value="">{t.create.scratchPick}</option>
                  {SCRATCH_SIZES.map((b) => <option key={b} value={String(b)}>{b / 1024 ** 3} GB</option>)}
                </select>
              </Field>
            ) : null}
            <Button small kind="quiet" onClick={() => onChange(rows.filter((_, j) => j !== i))}>{t.create.remove}</Button>
          </div>
        );
      })}
    </>
  );
}

/** The live half: folders and files of any machine's volumes, served through the daemon. */
export function LiveMounts(props: { draft: MountsDraft; onChange: (d: MountsDraft) => void; machines: MachineView[] }) {
  const { draft, onChange, machines } = props;
  const any = machines.some((m) => m.volumes.length > 0);
  return (
    <>
      <MountRows live rows={draft.folders} onChange={(folders) => onChange({ ...draft, folders })} machines={machines} inPlaceholder="projects/site" />
      <MountRows live rows={draft.files} onChange={(files) => onChange({ ...draft, files })} machines={machines} inPlaceholder="notes/ideas.md" />
      {scratchMissing(draft, machines) ? <p className="warn small">{t.create.scratchNote}</p> : null}
      <div className="row-actions">
        <Button small kind="quiet" onClick={() => onChange({ ...draft, folders: [...draft.folders, newRow(machines)] })} disabled={!any}>+ {t.create.addShare}</Button>
        <Button small kind="quiet" onClick={() => onChange({ ...draft, files: [...draft.files, { ...newRow(machines), to: "" }] })} disabled={!any}>+ {t.create.addFileShare}</Button>
      </div>
    </>
  );
}

/**
 * An agent's mounts, changed with `agents::edit`: the three lists stated
 * anew, refused while it works. Only for an agent this app made: the
 * daemon does not report mounts, so an agent made elsewhere has none here.
 */
export function MountsPanel(props: { name: string; active: boolean; onClose: () => void }) {
  const [view, setView] = useState<MountsView | null | "loading">("loading");
  const [machines, setMachines] = useState<MachineView[]>([]);
  const [draft, setDraft] = useState<MountsDraft>(emptyDraft());
  const [state, setState] = useState<{ kind: "idle" | "saving" | "saved" | "problem"; message?: string }>({ kind: "idle" });

  useEffect(() => {
    api.agentsMounts(props.name).then((v) => {
      setView(v);
      if (v) setDraft(draftOf(v));
    });
    api.machines().then(setMachines);
  }, [props.name]);

  if (view === "loading") return null;
  const pinned = view?.pinned ?? null;
  const home = pinned ? machines.find((m) => identityKey(m.identity) === identityKey(pinned)) : undefined;

  const save = async () => {
    setState({ kind: "saving" });
    try {
      const out = await api.agentsEdit({ name: props.name, ...inputsOf(draft, machines, pinned) });
      if (out.outcome === "edited") setState({ kind: "saved", message: t.mounts.saved });
      else setState({ kind: "problem", message: out.outcome === "active" ? t.mounts.active : out.outcome === "not_found" ? t.mounts.notFound : out.message });
    } catch (e) {
      setState({ kind: "problem", message: errorText(e) });
    }
  };

  return (
    <section className="mounts-panel">
      <header className="mounts-head">
        <h2>{t.mounts.title}</h2>
        <Button small kind="quiet" onClick={props.onClose}>{t.mounts.close}</Button>
      </header>
      {!view ? (
        <p className="muted small">{t.mounts.unknown}</p>
      ) : (
        <>
          <h3 className="list-head">{t.mounts.own}</h3>
          {pinned ? (
            <>
              <p className="muted small">{t.mounts.pinnedTo} {providerName(pinned)}</p>
              <MountRows rows={draft.own} onChange={(own) => setDraft({ ...draft, own })} machines={machines} fixed={home} inPlaceholder="(all of it)" />
              {home && home.volumes.length > 0 ? (
                <div className="row-actions">
                  <Button small kind="quiet" onClick={() => setDraft({ ...draft, own: [...draft.own, newRow(machines, home)] })}>+ {t.create.addStorage}</Button>
                </div>
              ) : null}
            </>
          ) : (
            <p className="muted small">{t.mounts.anywhere}</p>
          )}
          <h3 className="list-head">{t.mounts.live}</h3>
          <LiveMounts draft={draft} onChange={setDraft} machines={machines} />
          <footer className="mounts-foot">
            <span className="muted small">{props.active ? t.mounts.active : t.mounts.replaces}</span>
            {state.kind === "saved" ? <span className="ok small">{state.message}</span> : null}
            {state.kind === "problem" ? <span className="bad small">{state.message}</span> : null}
            <Button small kind="primary" onClick={save} disabled={props.active || state.kind === "saving" || scratchMissing(draft, machines) > 0}>{state.kind === "saving" ? t.mounts.saving : t.mounts.save}</Button>
          </footer>
        </>
      )}
    </section>
  );
}

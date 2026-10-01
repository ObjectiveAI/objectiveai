import { useCallback, useEffect, useState } from "react";
import type { FileNode } from "../bindings/FileNode";
import type { MachineView } from "../bindings/MachineView";
import type { ProviderView } from "../bindings/ProviderView";
import type { VolumeMode } from "../bindings/VolumeMode";
import type { VolumeView } from "../bindings/VolumeView";
import { Button, Chip, Empty, Field, Icon } from "../components/ui";
import { bytes as fmtBytes, identityKey, providerName } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";

const GB = 1024 ** 3;
type Open = { path: string; text: string; saved: string; size: number; binary: boolean; state: "idle" | "saving" | "saved" | "error"; error?: string };

const MODES: VolumeMode[] = ["persistent", "ephemeral", "read_only"];

/** Each machine's own volumes: pick a machine, then a volume. */
export function Storage() {
  const [machines, setMachines] = useState<MachineView[]>([]);
  const [machineKey, setMachineKey] = useState<string | null>(null);
  const [volumes, setVolumes] = useState<VolumeView[]>([]);
  const [used, setUsed] = useState<Record<string, number | "in-use">>({});
  const [room, setRoom] = useState<number | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [draft, setDraft] = useState<{ name: string; gb: number; mode: VolumeMode }>({ name: "", gb: 4, mode: "persistent" });

  useEffect(() => {
    api.machines().then((all) => {
      setMachines(all);
      setMachineKey((cur) => cur ?? (all[0] ? identityKey(all[0].identity) : null));
    });
  }, []);
  const machine = machines.find((m) => identityKey(m.identity) === machineKey);
  const on: ProviderView | null = machine?.identity ?? null;

  const load = useCallback(async () => {
    const on = machines.find((m) => identityKey(m.identity) === machineKey)?.identity;
    if (!on) return;
    setProblem(null);
    const listed = await api.volumes(on);
    if (listed.outcome === "error") {
      setVolumes([]);
      return setProblem(listed.message);
    }
    setVolumes(listed.volumes);
    const room = await api.volumeRoom(on);
    setRoom(room.outcome === "capacity" ? room.bytes : null);
    const stats: Record<string, number | "in-use"> = {};
    for (const v of listed.volumes) {
      const s = await api.volumeStat(on, v.name);
      stats[v.name] = s.outcome === "stat" ? s.bytes_used : "in-use";
    }
    setUsed(stats);
    setSelected((cur) => (cur && listed.volumes.some((v) => v.name === cur) ? cur : listed.volumes[0]?.name ?? null));
  }, [machines, machineKey]);
  useEffect(() => {
    load();
  }, [load]);

  const create = async () => {
    if (!on) return;
    setProblem(null);
    const out = await api.volumeCreate(on, draft.name.trim(), Math.round(draft.gb * GB), draft.mode);
    if (out.outcome === "done") {
      setSelected(draft.name.trim());
      setDraft({ name: "", gb: 4, mode: "persistent" });
      load();
    } else setProblem(out.outcome === "not_enough_room" ? t.storage.notEnoughRoom : out.outcome === "error" ? out.message : t.storage.inUse);
  };

  const volume = volumes.find((v) => v.name === selected);
  return (
    <div className="files">
      <aside className="files-tree">
        <header className="pane-head">
          <h1>{t.storage.title}</h1>
          <p className="muted small">{t.storage.note}</p>
        </header>
        {machines.length === 0 ? <p className="muted small machine-pick">{t.storage.noMachines}</p> : (
          <div className="machine-pick">
            <Field label={t.storage.machine}>
              <select value={machineKey ?? ""} onChange={(e) => { setMachineKey(e.target.value); setSelected(null); }}>
                {machines.map((m) => (
                  <option key={identityKey(m.identity)} value={identityKey(m.identity)}>{providerName(m.identity)}</option>
                ))}
              </select>
            </Field>
          </div>
        )}
        <ul className="volume-list">
          {volumes.map((v) => {
            const u = used[v.name];
            return (
              <li key={v.name}>
                <button className={`volume-row${selected === v.name ? " on" : ""}`} onClick={() => setSelected(v.name)}>
                  <span className="volume-name">{v.name} <span className="muted small">· {t.storage.modes[v.mode]}</span></span>
                  <span className="muted small">
                    {u === "in-use" ? t.storage.inUse.split(",")[0] : `${fmtBytes(u ?? 0)} ${t.storage.of} ${fmtBytes(v.bytes)}`}
                  </span>
                  {u !== "in-use" && typeof u === "number" ? <span className="meter"><span style={{ width: `${Math.min(100, (u / v.bytes) * 100)}%` }} /></span> : null}
                </button>
              </li>
            );
          })}
        </ul>
        <div className="new-volume">
          <h3 className="list-head">{t.storage.newVolume}</h3>
          <Field label={t.storage.name}>
            <input value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} spellCheck={false} />
          </Field>
          <Field label={t.storage.size} hint={room !== null ? `${t.storage.room}: ${fmtBytes(room)}` : undefined}>
            <input type="number" min={0.25} step={0.25} value={draft.gb} onChange={(e) => setDraft({ ...draft, gb: Number(e.target.value) })} />
          </Field>
          <Field label={t.storage.mode} hint={t.storage.modeNotes[draft.mode]}>
            <select value={draft.mode} onChange={(e) => setDraft({ ...draft, mode: e.target.value as VolumeMode })}>
              {MODES.map((m) => <option key={m} value={m}>{t.storage.modes[m]}</option>)}
            </select>
          </Field>
          <Button small kind="primary" onClick={create} disabled={!draft.name.trim() || !on}>{t.storage.create}</Button>
          {problem ? <p className="bad small">{problem}</p> : null}
        </div>
      </aside>
      {volume && on ? <VolumePane key={`${machineKey}/${volume.name}`} on={on} volume={volume} used={used[volume.name]} onChanged={load} /> : <Empty title={t.storage.pick} />}
    </div>
  );
}

function VolumePane(props: { on: ProviderView; volume: VolumeView; used: number | "in-use" | undefined; onChanged: () => void }) {
  const { on, volume } = props;
  const [tree, setTree] = useState<FileNode[] | null>(null);
  const [treeProblem, setTreeProblem] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set(["/projects", "/notes", "/projects/site"]));
  const [file, setFile] = useState<Open | null>(null);
  const [newPath, setNewPath] = useState("");
  const [gb, setGb] = useState(volume.bytes / GB);
  const [message, setMessage] = useState<string | null>(null);
  const [confirm, setConfirm] = useState(false);

  const refresh = useCallback(async () => {
    const out = await api.volumeTree(on, volume.name);
    if (out.outcome === "tree") {
      setTree(out.nodes);
      setTreeProblem(null);
    } else setTreeProblem(out.message);
  }, [on, volume.name]);
  useEffect(() => {
    refresh();
  }, [refresh]);

  const say = (outcome: string, detail?: string) =>
    setMessage(outcome === "done" ? null : outcome === "not_enough_room" ? t.storage.notEnoughRoom : outcome === "more_content_than_that" ? t.storage.moreContent : outcome === "in_use" ? t.storage.inUse : detail ?? null);

  const edit = async (bytes: number | null, mode: VolumeMode | null) => {
    const out = await api.volumeEdit(on, volume.name, bytes, mode);
    say(out.outcome, out.outcome === "error" ? out.message : undefined);
    props.onChanged();
  };

  const open = async (path: string) => {
    const read = await api.volumeRead(on, volume.name, path);
    if (read.outcome === "error") setFile({ path, text: "", saved: "", size: 0, binary: false, state: "error", error: read.message });
    else if (read.outcome === "binary") setFile({ path, text: "", saved: "", size: read.bytes, binary: true, state: "idle" });
    else setFile({ path, text: read.text, saved: read.text, size: read.bytes, binary: false, state: "idle" });
  };

  const save = async () => {
    if (!file) return;
    setFile({ ...file, state: "saving" });
    try {
      const out = await api.volumeWrite(on, volume.name, file.path, file.text);
      setFile((f) => f && (out.outcome === "written" ? { ...f, saved: f.text, state: "saved", size: new TextEncoder().encode(f.text).length } : { ...f, state: "error", error: out.message }));
      refresh();
      props.onChanged();
    } catch (e) {
      setFile((f) => f && { ...f, state: "error", error: errorText(e) });
    }
  };

  const createFile = async () => {
    const path = "/" + newPath.trim().replace(/^\/+/, "");
    const out = await api.volumeWrite(on, volume.name, path, "");
    if (out.outcome === "written") {
      setNewPath("");
      await refresh();
      open(path);
    } else setMessage(out.message);
  };

  const toggle = (path: string) =>
    setExpanded((s) => {
      const next = new Set(s);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });

  const render = (nodes: FileNode[], prefix: string): React.ReactNode =>
    [...nodes]
      .sort((a, b) => (a.kind === "directory" ? 0 : 1) - (b.kind === "directory" ? 0 : 1) || a.name.localeCompare(b.name))
      .map((node) => {
        const path = `${prefix}/${node.name}`;
        if (node.kind === "directory") {
          const isOpen = expanded.has(path);
          return (
            <li key={path}>
              <button className="tree-row" onClick={() => toggle(path)}>
                <span className={`chev${isOpen ? " open" : ""}`}><Icon name="chevron" /></span>
                <Icon name="folder" /> <span>{node.name}</span>
              </button>
              {isOpen ? <ul>{render(node.children, path)}</ul> : null}
            </li>
          );
        }
        return (
          <li key={path}>
            <button className={`tree-row${file?.path === path ? " on" : ""}`} onClick={() => open(path)}>
              <span className="chev" />
              <Icon name={node.kind === "symlink" ? "link" : "file"} /> <span>{node.name}</span>
              {node.kind === "file" && node.size !== null ? <span className="tree-size muted">{fmtBytes(node.size)}</span> : null}
            </button>
          </li>
        );
      });

  const dirty = file && file.text !== file.saved;
  return (
    <section className="files-editor">
      <header className="volume-head">
        <div className="volume-title">
          <h2>{volume.name}</h2>
          {typeof props.used === "number" ? <span className="muted small">{fmtBytes(props.used)} {t.storage.used} {t.storage.of} {fmtBytes(volume.bytes)}</span> : props.used === "in-use" ? <Chip tone="warn">{t.storage.inUse.split(",")[0]}</Chip> : null}
        </div>
        <div className="volume-actions">
          <select value={volume.mode} onChange={(e) => edit(null, e.target.value as VolumeMode)} aria-label={t.storage.mode}>
            {MODES.map((m) => <option key={m} value={m}>{t.storage.modes[m]}</option>)}
          </select>
          <input className="size-input" type="number" min={0.25} step={0.25} value={gb} onChange={(e) => setGb(Number(e.target.value))} aria-label={t.storage.size} />
          <Button small kind="quiet" onClick={() => edit(Math.round(gb * GB), null)} disabled={Math.round(gb * GB) === volume.bytes}>{t.storage.resize}</Button>
          {confirm ? (
            <>
              <Button small kind="danger" onClick={async () => { const out = await api.volumeDelete(on, volume.name); say(out.outcome, out.outcome === "error" ? out.message : undefined); setConfirm(false); props.onChanged(); }}>{t.storage.deleteConfirm}</Button>
              <Button small kind="quiet" onClick={() => setConfirm(false)}>{t.storage.keep}</Button>
            </>
          ) : (
            <Button small kind="quiet" onClick={() => setConfirm(true)}>{t.storage.delete}</Button>
          )}
        </div>
      </header>
      <p className="muted small volume-mode-line">{t.storage.modeNotes[volume.mode]} {t.storage.youCanWrite}</p>
      {message ? <div className="banner banner-warn">{message}</div> : null}
      <div className="volume-body">
        <div className="volume-tree">
          <div className="volume-tree-head">
            <span className="muted small">{t.storage.snapshot}</span>
            <Button small kind="quiet" onClick={refresh}>{t.storage.refresh}</Button>
          </div>
          {treeProblem ? <p className="bad small">{treeProblem}</p> : null}
          {tree && tree.length === 0 ? <p className="muted small">{t.storage.empty}</p> : null}
          <ul className="tree">{tree ? render(tree, "") : null}</ul>
          <div className="new-file">
            <input className="mono" value={newPath} placeholder={t.storage.newFilePlaceholder} onChange={(e) => setNewPath(e.target.value)} spellCheck={false} />
            <Button small kind="quiet" onClick={createFile} disabled={!newPath.trim()}>{t.storage.createFile}</Button>
          </div>
        </div>
        <div className="volume-file">
          {!file ? (
            <Empty title={t.storage.open} />
          ) : (
            <>
              <header className="editor-head">
                <span className="mono selectable">{file.path}</span>
                <span className="muted small">{fmtBytes(file.size)}</span>
                <span className="spacer" />
                {file.state === "error" ? <span className="bad small">{file.error}</span> : null}
                {dirty ? <span className="warn small">{t.storage.unsaved}</span> : file.state === "saved" ? <span className="ok small">{t.storage.saved}</span> : null}
                <Button small kind="primary" onClick={save} disabled={!dirty || file.binary || file.state === "saving"}>{file.state === "saving" ? t.storage.saving : t.storage.save}</Button>
              </header>
              {file.binary ? (
                <Empty title={t.storage.binary} />
              ) : (
                <textarea
                  className="editor mono"
                  value={file.text}
                  spellCheck={false}
                  onChange={(e) => setFile({ ...file, text: e.target.value, state: "idle" })}
                  onKeyDown={(e) => {
                    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
                      e.preventDefault();
                      save();
                    }
                  }}
                />
              )}
            </>
          )}
        </div>
      </div>
    </section>
  );
}

import { daemon, shared } from "@diverge/sdk";
import { useEffect, useMemo, useState } from "react";
import { Button, Card, Chip, Escapes, Field, Icon, Row, Section } from "../components/ui";
import { app, errorText } from "../lib/app";
import { useShared } from "../lib/context";
import { type ListSpec, handle, useListing } from "../lib/daemon";
import { bytes, time } from "../lib/format";
import { type Identity, identityKey, machineName } from "../lib/machines";
import { t } from "../strings";
import { NotConnected } from "./NotConnected";

const volumes = daemon.endpoints.volumes;
type Volume = daemon.endpoints.volumes.list.server.response.Volume;
type VolumeRef = daemon.reference.Volume;
type Mode = Volume["mode"];
type Node = shared.filetree.response.Node;

const VOLUMES: ListSpec<Volume, daemon.endpoints.volumes.list.server.response.Frame> = {
  open: (h) => volumes.list.client.execute(h, {}),
  key: volumes.list.server.response.key,
  change: volumes.list.server.response.change,
};

const GB = 1024 ** 3;
const MODES: Mode[] = ["persistent", "ephemeral", "read_only"];

export function Storage() {
  const { connected } = useShared();
  const held = useListing(VOLUMES, connected);
  const [names, setNames] = useState<Record<string, string>>({});
  const [picked, setPicked] = useState<VolumeRef | null>(null);
  useEffect(() => {
    app.machineNames().then(setNames).catch(() => {});
  }, []);
  if (!connected) return <NotConnected title={t.storage.title} />;
  const volume = picked ? held.items.find((v) => identityKey(v.provider) === identityKey(picked.provider) && v.name === picked.name) ?? null : null;
  const byMachine = new Map<string, { identity: Identity; volumes: Volume[] }>();
  for (const v of held.items) {
    const key = identityKey(v.provider);
    const group = byMachine.get(key) ?? { identity: v.provider, volumes: [] };
    group.volumes.push(v);
    byMachine.set(key, group);
  }
  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.storage.title}</h1>
        <p className="muted">{t.storage.note}</p>
        {held.ended ? <Card tone="bad">{held.ended}</Card> : null}
        <div className="split">
          <div className="split-list">
            {held.listed && held.items.length === 0 ? <p className="muted">{t.storage.none}</p> : null}
            {[...byMachine.values()].map((group) => (
              <div key={identityKey(group.identity)} className="rail-section">
                <div className="rail-head">
                  <span>{machineName(names, group.identity)}</span>
                </div>
                {group.volumes.map((v) => (
                  <Row key={v.name} on={volume === v} onClick={() => setPicked({ provider: v.provider, name: v.name })} className="rail-item">
                    <Icon name="folder" />
                    <span className="rail-item-name">{v.name}</span>
                    <span className="muted">{bytes(v.bytes)}</span>
                  </Row>
                ))}
              </div>
            ))}
            <NewVolume machines={[...byMachine.values()].map((g) => g.identity)} names={names} />
          </div>
          <div className="split-detail">{volume ? <VolumeView volume={volume} onDeleted={() => setPicked(null)} /> : <p className="muted">{t.storage.pick}</p>}</div>
        </div>
      </div>
    </div>
  );
}

/** One volume: what it is, its tree, the file picked in it. */
function VolumeView({ volume, onDeleted }: { volume: Volume; onDeleted: () => void }) {
  const ref = useMemo<VolumeRef>(() => ({ provider: volume.provider, name: volume.name }), [volume.provider, volume.name]);
  const [tree, setTree] = useState<Node[] | null>(null);
  const [said, setSaid] = useState<string | null>(null);
  const [path, setPath] = useState<string[] | null>(null);
  const [generation, setGeneration] = useState(0);
  useEffect(() => {
    let stopped = false;
    setTree(null);
    setSaid(null);
    volumes.filetree.client
      .execute(handle, { volume: ref, path: [] })
      .then((answer) => {
        if (stopped) return;
        if (typeof answer === "object" && "tree" in answer) setTree(answer.tree);
        else setSaid(answer === "held" ? t.storage.inUse : answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
      })
      .catch((error) => {
        if (!stopped) setSaid(errorText(error));
      });
    return () => {
      stopped = true;
    };
  }, [ref, generation]);
  const mounted = (volume.agents?.length ?? 0) + (volume.tools?.length ?? 0);
  return (
    <div className="fields">
      <div className="head-row">
        <h2>{volume.name}</h2>
        <Chip tone="plain">{t.storage.modes[volume.mode]}</Chip>
        {mounted > 0 ? <Chip tone="warn">{t.storage.mounted} {mounted}</Chip> : null}
        <span className="muted">
          {bytes(volume.bytes)} · {time(volume.created)}
        </span>
        <Button kind="tertiary" small onClick={() => setGeneration((g) => g + 1)}>
          {t.storage.refresh}
        </Button>
        <DeleteVolume volume={ref} onDeleted={onDeleted} />
      </div>
      <p className="muted">{t.storage.modeNotes[volume.mode]}</p>
      {said ? <Card tone="bad">{said}</Card> : null}
      {tree ? (
        <div className="tree">
          {tree.length === 0 ? <p className="muted">{t.storage.empty}</p> : null}
          {tree.map((node) => (
            <TreeNode key={node.name} node={node} at={[node.name]} picked={path} onPick={setPath} />
          ))}
        </div>
      ) : null}
      <NewFile volume={ref} onMade={(p) => { setPath(p); setGeneration((g) => g + 1); }} />
      {path ? <FileView key={path.join("/")} volume={ref} path={path} /> : null}
    </div>
  );
}

function TreeNode({ node, at, picked, onPick }: { node: Node; at: string[]; picked: string[] | null; onPick: (p: string[]) => void }) {
  const [open, setOpen] = useState(false);
  const here = picked !== null && picked.join("/") === at.join("/");
  if (node.type === "directory") {
    return (
      <div>
        <Row className="tree-row" onClick={() => setOpen((o) => !o)}>
          <Icon name={open ? "chevron" : "folder"} /> {node.name}
        </Row>
        {open ? (
          <div className="tree-children">
            {node.children.map((child) => (
              <TreeNode key={child.name} node={child} at={[...at, child.name]} picked={picked} onPick={onPick} />
            ))}
          </div>
        ) : null}
      </div>
    );
  }
  return (
    <Row className="tree-row" on={here} onClick={() => onPick(at)}>
      <Icon name={node.type === "symlink" ? "link" : "file"} /> {node.name} {node.type === "file" && node.size !== null ? <span className="muted">{bytes(node.size)}</span> : null}
    </Row>
  );
}

/** A file: read whole, shown as text when it is text, written back with Save. */
function FileView({ volume, path }: { volume: VolumeRef; path: string[] }) {
  const [text, setText] = useState<string | null>(null);
  const [binary, setBinary] = useState(false);
  const [said, setSaid] = useState<string | null>(null);
  const [state, setState] = useState<"reading" | "clean" | "dirty" | "saving" | "saved">("reading");
  useEffect(() => {
    let stopped = false;
    (async () => {
      try {
        const stream = await volumes.download.client.execute(handle, { volume, path });
        const pieces: Uint8Array[] = [];
        for await (const item of stream) {
          if (typeof item === "object" && "chunk" in item) pieces.push(item.chunk.body);
          else {
            setSaid(item === "held" ? t.storage.inUse : item === "forbidden" ? t.daemon.forbidden : item === "not_found" ? t.storage.gone : JSON.stringify(item));
            return;
          }
        }
        if (stopped) return;
        const whole = new Uint8Array(pieces.reduce((n, p) => n + p.length, 0));
        let at = 0;
        for (const p of pieces) {
          whole.set(p, at);
          at += p.length;
        }
        try {
          setText(new TextDecoder("utf-8", { fatal: true }).decode(whole));
          setState("clean");
        } catch {
          setBinary(true);
        }
      } catch (error) {
        if (!stopped) setSaid(errorText(error));
      }
    })();
    return () => {
      stopped = true;
    };
  }, [volume, path]);
  const save = async () => {
    if (text === null) return;
    setState("saving");
    setSaid(null);
    try {
      const body = new TextEncoder().encode(text);
      const answer = await volumes.upload.client.execute(handle, { kind: "file", volume, path }, () => body);
      if (answer === "uploaded") setState("saved");
      else {
        setState("dirty");
        setSaid(answer === "held" ? t.storage.inUse : answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
      }
    } catch (error) {
      setState("dirty");
      setSaid(errorText(error));
    }
  };
  return (
    <div className="fields">
      <div className="head-row">
        <h3 className="mono">{path.join("/")}</h3>
        {state === "dirty" ? <span className="muted">{t.storage.unsaved}</span> : state === "saved" ? <span className="muted">{t.storage.saved}</span> : null}
        {text !== null ? (
          <Button kind="primary" small onClick={() => void save()} disabled={state !== "dirty"}>
            {state === "saving" ? t.storage.saving : t.storage.save}
          </Button>
        ) : null}
      </div>
      {said ? <Card tone="bad">{said}</Card> : null}
      {binary ? <p className="muted">{t.storage.binary}</p> : null}
      {text !== null ? (
        <textarea
          className="mono"
          rows={16}
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            setState("dirty");
          }}
        />
      ) : null}
    </div>
  );
}

/** A new file at a path, empty, through an upload. */
function NewFile({ volume, onMade }: { volume: VolumeRef; onMade: (path: string[]) => void }) {
  const [name, setName] = useState("");
  const [said, setSaid] = useState<string | null>(null);
  const create = async () => {
    const path = name.split("/").filter(Boolean);
    if (!path.length) return;
    setSaid(null);
    try {
      const answer = await volumes.upload.client.execute(handle, { kind: "file", volume, path }, () => new Uint8Array());
      if (answer === "uploaded") {
        setName("");
        onMade(path);
      } else setSaid(answer === "held" ? t.storage.inUse : answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
    } catch (error) {
      setSaid(errorText(error));
    }
  };
  return (
    <div className="toolbar">
      <input value={name} onChange={(e) => setName(e.target.value)} placeholder={t.storage.newFilePlaceholder} />
      <Button kind="secondary" small onClick={() => void create()} disabled={!name.trim()}>
        {t.storage.createFile}
      </Button>
      {said ? <span className="muted">{said}</span> : null}
    </div>
  );
}

function DeleteVolume({ volume, onDeleted }: { volume: VolumeRef; onDeleted: () => void }) {
  const [asking, setAsking] = useState(false);
  const [said, setSaid] = useState<string | null>(null);
  const remove = async () => {
    setAsking(false);
    try {
      const answer = await volumes.delete.client.execute(handle, { volume });
      if (answer === "deleted") onDeleted();
      else setSaid(answer === "in_use" ? t.storage.inUse : answer === "not_found" ? t.storage.gone : answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
    } catch (error) {
      setSaid(errorText(error));
    }
  };
  if (asking) {
    return (
      <Escapes onEscape={() => setAsking(false)} className="toolbar">
        <Button kind="danger" small onClick={() => void remove()}>
          {t.storage.deleteConfirm}
        </Button>
        <Button kind="tertiary" small onClick={() => setAsking(false)}>
          {t.storage.keep}
        </Button>
      </Escapes>
    );
  }
  return (
    <span className="toolbar">
      {said ? <span className="muted">{said}</span> : null}
      <Button kind="tertiary" small onClick={() => setAsking(true)}>
        {t.storage.delete}
      </Button>
    </span>
  );
}

/** A volume on one machine: its name, its size, what agents' changes do. */
function NewVolume({ machines, names }: { machines: Identity[]; names: Record<string, string> }) {
  const [open, setOpen] = useState(false);
  const [machine, setMachine] = useState("");
  const [name, setName] = useState("");
  const [size, setSize] = useState("1");
  const [mode, setMode] = useState<Mode>("persistent");
  const [said, setSaid] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const identity = machines.find((m) => identityKey(m) === machine) ?? machines[0];
  const create = async () => {
    if (!identity) return;
    setBusy(true);
    setSaid(null);
    try {
      const answer = await volumes.create.client.execute(handle, { provider: identity, name: name.trim(), bytes: Math.round(Number(size) * GB), mode });
      if (answer === "created") {
        setName("");
        setOpen(false);
      } else setSaid(answer === "insufficient_capacity" ? t.storage.notEnoughRoom : answer === "exists" ? t.storage.exists : answer === "no_provider" ? t.storage.noProvider : answer === "unsupported" ? t.storage.unsupported : answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
    } catch (error) {
      setSaid(errorText(error));
    } finally {
      setBusy(false);
    }
  };
  if (!open) {
    return (
      <Button kind="secondary" small onClick={() => setOpen(true)} disabled={machines.length === 0} title={machines.length === 0 ? t.storage.noMachines : undefined}>
        <Icon name="plus" /> {t.storage.newVolume}
      </Button>
    );
  }
  return (
    <Escapes onEscape={() => setOpen(false)}>
      <Section title={t.storage.newVolume}>
        <div className="fields">
          <Field label={t.storage.machine}>
            <select value={identity ? identityKey(identity) : ""} onChange={(e) => setMachine(e.target.value)}>
              {machines.map((m) => (
                <option key={identityKey(m)} value={identityKey(m)}>
                  {machineName(names, m)}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t.storage.name} needed>
            <input value={name} onChange={(e) => setName(e.target.value)} />
          </Field>
          <Field label={t.storage.size}>
            <input type="number" min={0} step="0.5" value={size} onChange={(e) => setSize(e.target.value)} />
          </Field>
          <Field label={t.storage.mode}>
            <select value={mode} onChange={(e) => setMode(e.target.value as Mode)}>
              {MODES.map((m) => (
                <option key={m} value={m}>
                  {t.storage.modes[m]}
                </option>
              ))}
            </select>
          </Field>
          {said ? <Card tone="bad">{said}</Card> : null}
          <div className="toolbar">
            <Button kind="primary" small onClick={() => void create()} disabled={busy || !name.trim() || !identity}>
              {t.storage.create}
            </Button>
            <Button kind="tertiary" small onClick={() => setOpen(false)}>
              {t.common.cancel}
            </Button>
          </div>
        </div>
      </Section>
    </Escapes>
  );
}

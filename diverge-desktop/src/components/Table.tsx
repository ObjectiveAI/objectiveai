import { useCallback, useEffect, useState } from "react";
import type { FileNode } from "../bindings/FileNode";
import type { SpaceSummary } from "../bindings/SpaceSummary";
import { bytes as fmtBytes, spaceTitle } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";
import { Button, Empty, Icon } from "./ui";

type Open = { path: string; text: string; saved: string; binary: boolean; size: number; state: "idle" | "saving" | "saved" | "error"; error?: string };

/** A room's table: the room container's files, the same for everyone in the room. */
export function Table(props: { space: SpaceSummary; rooms: SpaceSummary[] }) {
  const id = props.space.id;
  const [nodes, setNodes] = useState<FileNode[] | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [file, setFile] = useState<Open | null>(null);
  const [newPath, setNewPath] = useState("");
  const [moveTo, setMoveTo] = useState("");
  const [moved, setMoved] = useState<string | null>(null);
  const [exists, setExists] = useState<string | null>(null);
  const [replacing, setReplacing] = useState(false);

  const refresh = useCallback(async () => {
    const out = await api.tableTree(id);
    if (out.outcome === "tree") {
      setNodes(out.nodes);
      setProblem(null);
    } else setProblem(out.message);
  }, [id]);
  useEffect(() => {
    refresh();
  }, [refresh]);

  const open = async (path: string) => {
    setMoved(null);
    const read = await api.tableRead(id, path);
    if (read.outcome === "error") setFile({ path, text: "", saved: "", binary: false, size: 0, state: "error", error: read.message });
    else if (read.outcome === "binary") setFile({ path, text: "", saved: "", binary: true, size: read.bytes, state: "idle" });
    else setFile({ path, text: read.text, saved: read.text, binary: false, size: read.bytes, state: "idle" });
  };

  const save = async () => {
    if (!file) return;
    setFile({ ...file, state: "saving" });
    try {
      const out = await api.tableWrite(id, file.path, file.text);
      setFile((f) => f && (out.outcome === "written" ? { ...f, saved: f.text, state: "saved" } : { ...f, state: "error", error: out.message }));
      refresh();
    } catch (e) {
      setFile((f) => f && { ...f, state: "error", error: errorText(e) });
    }
  };

  /** Whether a path is already on the table. */
  const has = (path: string): boolean => {
    const walk = (list: FileNode[], prefix: string): boolean => list.some((n) => `${prefix}${n.name}` === path || (n.kind === "directory" && walk(n.children ?? [], `${prefix}${n.name}/`)));
    return walk(nodes ?? [], "");
  };

  const create = async () => {
    const path = newPath.trim().replace(/^\/+/, "");
    // Never wipe what's there: offer to open it instead.
    if (has(path)) {
      setExists(path);
      return;
    }
    const out = await api.tableWrite(id, path, "");
    if (out.outcome === "written") {
      setNewPath("");
      await refresh();
      open(path);
    } else setProblem(out.message);
  };

  const copy = async (replace = false) => {
    if (!file || !moveTo) return;
    // Ask before replacing a file of the same name on the other table.
    if (!replace) {
      const there = await api.tableRead(moveTo, file.path);
      if (there.outcome !== "error") {
        setReplacing(true);
        return;
      }
    }
    setReplacing(false);
    const out = await api.tableTransfer(id, file.path, moveTo);
    setMoved(out.outcome === "written" ? t.spaces.copied2 : out.message);
  };

  const toggle = (path: string) =>
    setExpanded((s) => {
      const next = new Set(s);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });

  const render = (list: FileNode[], prefix: string, depth: number): React.ReactNode =>
    [...list]
      .sort((a, b) => (a.kind === "directory" ? 0 : 1) - (b.kind === "directory" ? 0 : 1) || a.name.localeCompare(b.name))
      .map((node) => {
        const path = prefix ? `${prefix}/${node.name}` : node.name;
        const pad = { paddingLeft: 10 + depth * 14 };
        if (node.kind === "directory") {
          const isOpen = expanded.has(path);
          return (
            <li key={path}>
              <button className="tree-row" style={pad} onClick={() => toggle(path)}>
                <span className={`chev${isOpen ? " open" : ""}`}><Icon name="chevron" /></span>
                <Icon name="folder" /> <span>{node.name}</span>
              </button>
              {isOpen ? <ul>{render(node.children, path, depth + 1)}</ul> : null}
            </li>
          );
        }
        return (
          <li key={path}>
            <button className={`tree-row${file?.path === path ? " on" : ""}`} style={pad} onClick={() => open(path)}>
              <span className="chev" />
              <Icon name="file" /> <span>{node.name}</span>
              {node.kind === "file" && node.size !== null ? <span className="tree-size muted">{fmtBytes(node.size)}</span> : null}
            </button>
          </li>
        );
      });

  const others = props.rooms.filter((r) => r.id !== id);
  const dirty = file && file.text !== file.saved;
  return (
    <div className="table">
      <div className="table-tree">
        <p className="muted small">{t.spaces.tableNote}</p>
        {problem ? <p className="bad small">{problem}</p> : null}
        {nodes && nodes.length === 0 ? <p className="muted small">{t.spaces.tableEmpty}</p> : null}
        <ul className="tree">{nodes ? render(nodes, "", 0) : null}</ul>
        <div className="new-file">
          <input className="mono" value={newPath} placeholder={t.spaces.newFilePlaceholder} onChange={(e) => setNewPath(e.target.value)} spellCheck={false} />
          <Button small kind="quiet" onClick={create} disabled={!newPath.trim()}>{t.spaces.create}</Button>
          {exists ? (
            <span className="warn small">
              {exists} {t.spaces.alreadyThere}{" "}
              <button className="link small" onClick={() => { open(exists); setExists(null); setNewPath(""); }}>{t.spaces.openIt}</button>
            </span>
          ) : null}
        </div>
      </div>
      <div className="table-file">
        {!file ? (
          <Empty title={t.spaces.pick} />
        ) : (
          <>
            <header className="editor-head">
              <span className="mono selectable">{file.path}</span>
              <span className="muted small">{fmtBytes(file.size)}</span>
              <span className="spacer" />
              {file.state === "error" ? <span className="bad small">{file.error}</span> : null}
              {dirty ? <span className="warn small">{t.storage.unsaved}</span> : file.state === "saved" ? <span className="ok small">{t.spaces.saved}</span> : null}
              <Button small kind="primary" onClick={save} disabled={!dirty || file.binary || file.state === "saving"}>{t.spaces.save}</Button>
            </header>
            {file.binary ? (
              <Empty title={t.storage.binary} />
            ) : (
              <textarea className="editor mono" value={file.text} spellCheck={false} onChange={(e) => setFile({ ...file, text: e.target.value, state: "idle" })} />
            )}
            {others.length ? (
              <footer className="table-copy">
                <select value={moveTo} onChange={(e) => setMoveTo(e.target.value)} aria-label={t.spaces.copyTo}>
                  <option value="">{t.spaces.copyTo}…</option>
                  {others.map((r) => <option key={r.id} value={r.id}>{spaceTitle(r)}</option>)}
                </select>
                <Button small kind="quiet" onClick={() => copy()} disabled={!moveTo}>{t.spaces.copyTo.split(" ")[0]}</Button>
                {replacing ? (
                  <span className="warn small">
                    {t.spaces.replaceThere}{" "}
                    <Button small kind="danger" onClick={() => copy(true)}>{t.spaces.replaceIt}</Button>
                    <Button small kind="quiet" onClick={() => setReplacing(false)}>{t.spaces.keepTheirs}</Button>
                  </span>
                ) : null}
                {moved ? <span className={moved === t.spaces.copied2 ? "ok small" : "warn small"}>{moved}</span> : null}
              </footer>
            ) : null}
          </>
        )}
      </div>
    </div>
  );
}

import { useEffect, useState } from "react";
import type { FileNode } from "../bindings/FileNode";
import { Button, Empty, Icon } from "../components/ui";
import { bytes } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { applyTree } from "../lib/tree";
import { t } from "../strings";

type Open = { path: string; text: string; saved: string; size: number; binary: boolean; state: "idle" | "saving" | "saved" | "error"; error?: string };

export function Files() {
  const [tree, setTree] = useState<FileNode[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set(["/projects", "/notes"]));
  const [file, setFile] = useState<Open | null>(null);
  const [newPath, setNewPath] = useState("");

  useEffect(() => {
    let scope: string | null = null;
    let closed = false;
    api
      .treeOpen("/", (event) => {
        if (event.event === "error") setProblem(event.message);
        else setTree((current) => applyTree(current, event));
      })
      .then((id) => {
        scope = id;
        if (closed) api.scopeClose(id);
      });
    return () => {
      closed = true;
      if (scope) api.scopeClose(scope);
    };
  }, []);

  const openFile = async (path: string) => {
    const read = await api.fileRead(path);
    if (read.outcome === "error") setFile({ path, text: "", saved: "", size: 0, binary: false, state: "error", error: read.message });
    else if (read.outcome === "binary") setFile({ path, text: "", saved: "", size: read.bytes, binary: true, state: "idle" });
    else setFile({ path, text: read.text, saved: read.text, size: read.bytes, binary: false, state: "idle" });
  };

  const save = async () => {
    if (!file) return;
    setFile({ ...file, state: "saving" });
    try {
      const written = await api.fileWrite(file.path, file.text);
      setFile((f) => f && (written.outcome === "written" ? { ...f, saved: f.text, state: "saved", size: new TextEncoder().encode(f.text).length } : { ...f, state: "error", error: written.message }));
    } catch (e) {
      setFile((f) => f && { ...f, state: "error", error: errorText(e) });
    }
  };

  const create = async () => {
    const path = newPath.trim();
    if (!path.startsWith("/")) return;
    const written = await api.fileWrite(path, "");
    if (written.outcome === "written") {
      setNewPath("");
      openFile(path);
    } else setProblem(written.message);
  };

  const toggle = (path: string) => setExpanded((s) => {
    const next = new Set(s);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    return next;
  });

  const render = (nodes: FileNode[], prefix: string, depth: number) =>
    [...nodes]
      .sort((a, b) => (a.kind === "directory" ? 0 : 1) - (b.kind === "directory" ? 0 : 1) || a.name.localeCompare(b.name))
      .map((node) => {
        const path = `${prefix}/${node.name}`;
        const pad = { paddingLeft: 10 + depth * 14 };
        if (node.kind === "directory") {
          const isOpen = expanded.has(path);
          return (
            <li key={path}>
              <button className="tree-row" style={pad} onClick={() => toggle(path)} title={node.watched ? undefined : t.files.stale}>
                <span className={`chev${isOpen ? " open" : ""}`}><Icon name="chevron" /></span>
                <Icon name="folder" /> <span>{node.name}</span>
              </button>
              {isOpen ? <ul>{render(node.children, path, depth + 1)}</ul> : null}
            </li>
          );
        }
        return (
          <li key={path}>
            <button className={`tree-row${file?.path === path ? " on" : ""}`} style={pad} onClick={() => openFile(path)}>
              <span className="chev" />
              <Icon name={node.kind === "symlink" ? "link" : "file"} /> <span>{node.name}</span>
              {node.kind === "file" && node.size !== null ? <span className="tree-size muted">{bytes(node.size)}</span> : null}
            </button>
          </li>
        );
      });

  const dirty = file && file.text !== file.saved;
  return (
    <div className="files">
      <aside className="files-tree">
        <header className="pane-head">
          <h1>{t.files.title}</h1>
          <p className="muted small">{t.files.note}</p>
        </header>
        {problem ? <div className="banner banner-bad">{problem}</div> : null}
        <ul className="tree">{render(tree, "", 0)}</ul>
        <div className="new-file">
          <input className="mono" value={newPath} placeholder={t.files.newFilePlaceholder} onChange={(e) => setNewPath(e.target.value)} spellCheck={false} />
          <Button small kind="quiet" onClick={create} disabled={!newPath.trim().startsWith("/")}>{t.files.create}</Button>
        </div>
      </aside>
      <section className="files-editor">
        {!file ? (
          <Empty title={t.files.open} />
        ) : (
          <>
            <header className="editor-head">
              <span className="mono selectable">{file.path}</span>
              <span className="muted small">{bytes(file.size)}</span>
              <span className="spacer" />
              {file.state === "error" ? <span className="bad small">{file.error}</span> : null}
              {dirty ? <span className="warn small">{t.files.unsaved}</span> : file.state === "saved" ? <span className="ok small">{t.files.saved}</span> : null}
              <Button small kind="primary" onClick={save} disabled={!dirty || file.binary || file.state === "saving"}>{file.state === "saving" ? t.files.saving : t.files.save}</Button>
            </header>
            {file.binary ? (
              <Empty title={t.files.binary} />
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
      </section>
    </div>
  );
}

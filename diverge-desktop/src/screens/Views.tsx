import { useEffect, useRef, useState } from "react";
import type { LogEvent } from "../bindings/LogEvent";
import type { LogsQuery } from "../bindings/LogsQuery";
import type { SavedView } from "../bindings/SavedView";
import { Button, Field, Section } from "../components/ui";
import { useShared } from "../lib/context";
import { argsLine } from "../lib/conversation";
import { time } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";

const KINDS = Object.keys(t.itemTypes);

const blank = (name: string): LogsQuery => ({ name, logs_index_from: null, logs_index_to: null, created_from: null, created_to: null, item_type: null, jq: null, count: null, watch: true });

const PRESETS: { title: string; query: (name: string) => LogsQuery }[] = [
  { title: t.views.presetNames.tools, query: (n) => ({ ...blank(n), item_type: "assistant_tool_call", jq: "{tool: .name, with: (.arguments | fromjson? // .)}" }) },
  { title: t.views.presetNames.errors, query: (n) => ({ ...blank(n), item_type: "error" }) },
  { title: t.views.presetNames.asked, query: (n) => ({ ...blank(n), item_type: "user_text_content", jq: ".text", watch: false }) },
  { title: t.views.presetNames.where, query: (n) => ({ ...blank(n), item_type: "active", jq: ".identity" }) },
];

type Row = { key: number; event: LogEvent };

function summary(event: LogEvent): string {
  if (event.event !== "entry") return "";
  const item = event.entry.item;
  switch (item.kind) {
    case "user_text":
    case "text":
    case "reasoning":
    case "refusal":
      return item.text;
    case "tool_call":
      return `${item.name} ${argsLine(item.arguments)}`;
    case "tool_response":
      return item.text.split("\n")[0];
    case "usage":
      return `${item.total_tokens} tokens`;
    case "error":
      return item.message;
    case "active":
    case "inactive":
      return item.provider.kind === "outgoing" ? item.provider.address : item.provider.identity;
    case "notification":
      return JSON.stringify(item.message);
    default:
      return item.kind;
  }
}

export function Views() {
  const { agents } = useShared();
  const [saved, setSaved] = useState<SavedView[]>([]);
  const [id, setId] = useState("");
  const [title, setTitle] = useState("");
  const [query, setQuery] = useState<LogsQuery>(blank(""));
  const [rows, setRows] = useState<Row[]>([]);
  const [state, setState] = useState<"idle" | "live" | "ended">("idle");
  const [problem, setProblem] = useState<string | null>(null);
  const scope = useRef<string | null>(null);
  const counter = useRef(0);

  useEffect(() => {
    api.views().then(setSaved);
    return () => {
      if (scope.current) api.scopeClose(scope.current);
    };
  }, []);
  useEffect(() => {
    if (!query.name && agents[0]) setQuery((q) => ({ ...q, name: agents[0].name }));
  }, [agents, query.name]);

  const run = async (q: LogsQuery = query) => {
    if (scope.current) api.scopeClose(scope.current);
    setRows([]);
    setProblem(null);
    setState(q.watch ? "live" : "idle");
    try {
      scope.current = await api.logsOpen(q, (event) => {
        if (event.event === "end") setState("ended");
        else if (event.event === "error") setProblem(event.message);
        else setRows((r) => [...r, { key: counter.current++, event }]);
      });
    } catch (e) {
      setProblem(errorText(e));
      setState("idle");
    }
  };

  const save = async () => {
    try {
      const view = await api.viewSave({ id, title: title || t.views.untitled, query, saved: "" });
      setId(view.id);
      setSaved(await api.views());
    } catch (e) {
      setProblem(errorText(e));
    }
  };

  const load = (v: SavedView) => {
    setId(v.id);
    setTitle(v.title);
    setQuery(v.query);
    run(v.query);
  };

  const set = (patch: Partial<LogsQuery>) => setQuery((q) => ({ ...q, ...patch }));

  return (
    <div className="views">
      <aside className="views-list">
        <header className="pane-head">
          <h1>{t.views.title}</h1>
          <p className="muted small">{t.views.note}</p>
        </header>
        <Button small kind="quiet" onClick={() => { setId(""); setTitle(""); setQuery(blank(agents[0]?.name ?? "")); setRows([]); setState("idle"); }}>+ {t.views.new}</Button>
        <h3 className="list-head">{t.views.saved}</h3>
        {saved.length === 0 ? <p className="muted small">{t.views.none}</p> : null}
        <ul>
          {saved.map((v) => (
            <li key={v.id}>
              <button className={`list-row${v.id === id ? " on" : ""}`} onClick={() => load(v)}>
                <span>{v.title}</span>
                <span className="muted small">{v.query.name}{v.query.item_type ? ` · ${t.itemTypes[v.query.item_type] ?? v.query.item_type}` : ""}</span>
              </button>
            </li>
          ))}
        </ul>
      </aside>
      <section className="views-main">
        <div className="page-inner">
          <Section title={title || t.views.untitled}>
            <div className="presets"><span className="muted small">{t.views.presets}</span>{PRESETS.map((p) => <Button key={p.title} small kind="quiet" onClick={() => { const q = p.query(query.name || agents[0]?.name || ""); setTitle(p.title); setId(""); setQuery(q); run(q); }}>{p.title}</Button>)}</div>
            <div className="row">
              <Field label={t.views.titleLabel}>
                <input value={title} placeholder={t.views.untitled} onChange={(e) => setTitle(e.target.value)} />
              </Field>
              <Field label={t.views.agent}>
                <select value={query.name} onChange={(e) => set({ name: e.target.value })}>
                  {agents.map((a) => <option key={a.name} value={a.name}>{a.name}</option>)}
                </select>
              </Field>
              <Field label={t.views.kind}>
                <select value={query.item_type ?? ""} onChange={(e) => set({ item_type: e.target.value || null })}>
                  <option value="">{t.views.everything}</option>
                  {KINDS.map((k) => <option key={k} value={k}>{t.itemTypes[k]}</option>)}
                </select>
              </Field>
            </div>
            <div className="row">
              <Field label={t.views.jq} size="grow">
                <input className="mono" value={query.jq ?? ""} placeholder={t.views.jqPlaceholder} onChange={(e) => set({ jq: e.target.value || null })} spellCheck={false} />
              </Field>
              <Field label={t.views.count} size="narrow">
                <input type="number" min={0} value={query.count ?? ""} onChange={(e) => set({ count: e.target.value === "" ? null : Number(e.target.value) })} />
              </Field>
              <Field label={t.views.from} size="narrow">
                <input type="number" min={1} value={query.logs_index_from ?? ""} onChange={(e) => set({ logs_index_from: e.target.value === "" ? null : Number(e.target.value) })} />
              </Field>
              <Field label={t.views.to} size="narrow">
                <input type="number" min={1} value={query.logs_index_to ?? ""} onChange={(e) => set({ logs_index_to: e.target.value === "" ? null : Number(e.target.value) })} />
              </Field>
            </div>
            <div className="row-actions">
              <label className="switch switch-line">
                <input type="checkbox" checked={query.watch} onChange={(e) => set({ watch: e.target.checked })} />
                <span>{t.views.watch}</span>
              </label>
              <span className="spacer" />
              {id ? <Button small kind="quiet" onClick={async () => { await api.viewDelete(id); setId(""); setSaved(await api.views()); }}>{t.views.delete}</Button> : null}
              <Button kind="quiet" onClick={save} disabled={!query.name}>{t.views.save}</Button>
              <Button kind="primary" onClick={() => run()} disabled={!query.name}>{t.views.run}</Button>
            </div>
          </Section>

          <div className="results-head">
            <h2>{t.views.results}</h2>
            <span className="muted small">{rows.length}</span>
            {state === "live" ? <span className="ok small">● {t.views.live}</span> : state === "ended" ? <span className="muted small">{t.views.ended}</span> : null}
          </div>
          {problem ? <div className="banner banner-bad">{problem}</div> : null}
          {rows.length === 0 && state !== "idle" ? <p className="muted">{t.views.noResults}</p> : null}
          <ul className="results">
            {rows.map(({ key, event }) =>
              event.event === "entry" ? (
                <li key={key} className="result">
                  <span className="result-index mono muted">#{event.entry.logs_index}</span>
                  <span className="result-kind">{event.entry.item.kind.replace(/_/g, " ")}</span>
                  <span className="result-text selectable">{summary(event)}</span>
                  <span className="muted small">{time(event.entry.created)}</span>
                </li>
              ) : event.event === "value" ? (
                <li key={key} className="result result-raw">
                  <pre className="selectable">{typeof event.value === "string" ? event.value : JSON.stringify(event.value, null, 2)}</pre>
                </li>
              ) : null,
            )}
          </ul>
        </div>
      </section>
    </div>
  );
}

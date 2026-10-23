import { daemon } from "@diverge/sdk";
import { useEffect, useMemo, useState } from "react";
import type { SavedView } from "../bindings/SavedView";
import { Button, Card, Field, Row, Section } from "../components/ui";
import { app, errorText } from "../lib/app";
import { useShared } from "../lib/context";
import { type LogItem, type LogsRequest, useLogs } from "../lib/conversation";
import { number, time } from "../lib/format";
import { t } from "../strings";
import { NotConnected } from "./NotConnected";

type ItemType = daemon.endpoints.agents.logs.client.request.ItemType;
const TYPES = daemon.endpoints.agents.logs.client.request.ItemType.options;

/** What a saved View keeps: the log request less the agent, and the agent it was last run on, by key. */
type Query = { type?: ItemType; count?: number; watch?: boolean; logs_index_from?: number; logs_index_to?: number; agent?: { template: string; index: number } };

function queryOf(value: unknown): Query {
  return typeof value === "object" && value !== null ? (value as Query) : {};
}

export function Views() {
  const { connected, agents } = useShared();
  const [saved, setSaved] = useState<SavedView[]>([]);
  const [title, setTitle] = useState(t.views.untitled);
  const [id, setId] = useState<string | null>(null);
  const [query, setQuery] = useState<Query>({});
  const [running, setRunning] = useState<LogsRequest | null>(null);
  const [said, setSaid] = useState<string | null>(null);
  useEffect(() => {
    app.views().then(setSaved).catch(() => {});
  }, []);
  const request = useMemo(() => running, [running]);
  const log = useLogs(request, connected);
  if (!connected) return <NotConnected title={t.views.title} />;
  const agent = query.agent ? agents.items.find((a) => a.template === query.agent!.template && a.index === query.agent!.index) : undefined;
  const run = () => {
    if (!query.agent) return;
    const { agent: who, ...rest } = query;
    setRunning({ agent: who, ...rest });
  };
  const save = () =>
    app
      .viewSave({ id: id ?? crypto.randomUUID(), title: title.trim() || t.views.untitled, query, saved: new Date().toISOString() })
      .then((kept) => {
        setId(kept.id);
        return app.views().then(setSaved);
      })
      .catch((error) => setSaid(errorText(error)));
  const load = (v: SavedView) => {
    setId(v.id);
    setTitle(v.title);
    setQuery(queryOf(v.query));
    setRunning(null);
  };
  const remove = (v: SavedView) =>
    app
      .viewDelete(v.id)
      .then(() => app.views().then(setSaved))
      .then(() => {
        if (id === v.id) setId(null);
      })
      .catch((error) => setSaid(errorText(error)));
  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.views.title}</h1>
        <p className="muted">{t.views.note}</p>
        <div className="split">
          <div className="split-list">
            <div className="rail-head">
              <span>{t.views.saved}</span>
            </div>
            {saved.length === 0 ? <p className="muted">{t.views.none}</p> : null}
            {saved.map((v) => (
              <Row key={v.id} className="rail-item" on={id === v.id} onClick={() => load(v)}>
                <span className="rail-item-name">{v.title}</span>
              </Row>
            ))}
            <Button
              kind="secondary"
              small
              onClick={() => {
                setId(null);
                setTitle(t.views.untitled);
                setQuery({});
                setRunning(null);
              }}
            >
              {t.views.new}
            </Button>
          </div>
          <div className="split-detail">
            <Section title={title}>
              <div className="fields">
                <Field label={t.views.titleLabel}>
                  <input value={title} onChange={(e) => setTitle(e.target.value)} />
                </Field>
                <Field label={t.views.agent} needed>
                  <select
                    value={agent ? `${agent.template}:${agent.index}` : ""}
                    onChange={(e) => {
                      const a = agents.items.find((x) => `${x.template}:${x.index}` === e.target.value);
                      setQuery((q) => ({ ...q, agent: a ? { template: a.template, index: a.index } : undefined }));
                    }}
                  >
                    <option value="">—</option>
                    {agents.items.map((a) => (
                      <option key={`${a.template}:${a.index}`} value={`${a.template}:${a.index}`}>
                        {a.name ?? `${t.rail.unnamed} ${a.index}`}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label={t.views.kind}>
                  <select value={query.type ?? ""} onChange={(e) => setQuery((q) => ({ ...q, type: (e.target.value || undefined) as ItemType | undefined }))}>
                    <option value="">{t.views.everything}</option>
                    {TYPES.map((k) => (
                      <option key={k} value={k}>
                        {t.itemTypes[k] ?? k}
                      </option>
                    ))}
                  </select>
                </Field>
                <div className="toolbar">
                  <Field label={t.views.from}>
                    <input type="number" min={0} value={query.logs_index_from ?? ""} onChange={(e) => setQuery((q) => ({ ...q, logs_index_from: e.target.value === "" ? undefined : Number(e.target.value) }))} />
                  </Field>
                  <Field label={t.views.to}>
                    <input type="number" min={0} value={query.logs_index_to ?? ""} onChange={(e) => setQuery((q) => ({ ...q, logs_index_to: e.target.value === "" ? undefined : Number(e.target.value) }))} />
                  </Field>
                  <Field label={t.views.count}>
                    <input type="number" min={0} value={query.count ?? ""} onChange={(e) => setQuery((q) => ({ ...q, count: e.target.value === "" ? undefined : Number(e.target.value) }))} />
                  </Field>
                </div>
                <label className="switch">
                  <input type="checkbox" checked={query.watch ?? false} onChange={(e) => setQuery((q) => ({ ...q, watch: e.target.checked || undefined }))} /> {t.views.watch}
                </label>
                {said ? <Card tone="bad">{said}</Card> : null}
                <div className="toolbar">
                  <Button kind="primary" small onClick={run} disabled={!query.agent}>
                    {t.views.run}
                  </Button>
                  <Button kind="secondary" small onClick={() => void save()}>
                    {t.views.save}
                  </Button>
                  {id ? (
                    <Button kind="tertiary" small onClick={() => void remove(saved.find((v) => v.id === id)!)}>
                      {t.views.delete}
                    </Button>
                  ) : null}
                </div>
              </div>
            </Section>
            {running ? (
              <Section title={t.views.results} note={log.ended ?? (log.done ? t.views.ended : running.watch ? t.views.live : undefined)}>
                {log.turns.length === 0 ? <p className="muted">{t.views.noResults}</p> : null}
                <div className="lines">
                  {log.turns.map(({ item }) => (
                    <div key={item.logs_index} className="line">
                      <span className="mono muted">{number(item.logs_index)}</span> <span className="muted">{time(item.created)}</span> <strong>{t.itemTypes[item.type] ?? item.type}</strong> <span className="pre selectable">{summary(item)}</span>
                    </div>
                  ))}
                </div>
              </Section>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}

/** One line of an item: its words where it has them, else its JSON. */
function summary(item: LogItem): string {
  if ("text" in item && typeof item.text === "string") return item.text;
  if (item.type === "assistant_tool_call") return `${item.name} ${item.arguments ?? ""}`;
  if (item.type === "usage") return `${item.total_tokens} ${t.convo.tokens}`;
  const { logs_index: _i, created: _c, type: _t, ...rest } = item;
  return JSON.stringify(rest);
}

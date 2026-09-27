import { useCallback, useEffect, useMemo, useState } from "react";
import type { HomeMove } from "../bindings/HomeMove";
import { Markdown } from "../components/Markdown";
import { Button, Chip, Dot } from "../components/ui";
import { useShared } from "../lib/context";
import { ago, kindTitle, providerName, time } from "../lib/format";
import { api } from "../lib/ipc";
import { t } from "../strings";

const VIEWS: Record<string, (m: HomeMove) => boolean> = {
  all: (m) => !m.entry.parent && m.entry.kind !== "say" && m.space.kind !== "dm",
  asks: (m) => m.entry.kind === "ask",
  shows: (m) => m.entry.kind === "show",
  board: (m) => ["task", "offering"].includes(m.entry.kind) && !m.entry.parent,
  runs: (m) => m.entry.kind === "run",
  ideas: (m) => ["direction", "synthesis"].includes(m.entry.kind),
};

export function Home() {
  const { agents, open, cards, knocks, answerKnock, homeId } = useShared();
  const [feed, setFeed] = useState<HomeMove[]>([]);
  const [view, setView] = useState("all");
  const [draft, setDraft] = useState("");

  const load = useCallback(async () => setFeed(await api.homeFeed()), []);
  useEffect(() => {
    load();
    const timer = setInterval(load, 4000);
    return () => clearInterval(timer);
  }, [load]);

  const post = async (verb: "show" | "ask") => {
    const text = draft.trim();
    if (!text || !homeId) return;
    await api.spaceCall(homeId, verb, verb === "show" ? { title: text } : { what: text });
    setDraft("");
    load();
  };

  const shown = useMemo(() => feed.filter(VIEWS[view] ?? VIEWS.all), [feed, view]);
  const today = new Date().toDateString();
  const doneToday = feed.filter((m) => m.entry.kind === "run" && new Date(m.entry.at).toDateString() === today);

  return (
    <div className="home">
      <section className="home-main">
        <div className="home-views" role="tablist">
          {Object.keys(VIEWS).map((k) => (
            <button key={k} role="tab" aria-selected={view === k} className={`home-view${view === k ? " on" : ""}`} onClick={() => setView(k)}>{t.home.views[k]}</button>
          ))}
        </div>
        {homeId ? (
          <div className="home-compose">
            <textarea rows={2} value={draft} placeholder={t.home.composePlaceholder} onChange={(e) => setDraft(e.target.value)} />
            <div className="home-compose-actions">
              <Button small kind="primary" onClick={() => post("show")} disabled={!draft.trim()}>{t.home.show}</Button>
              <Button small kind="plain" onClick={() => post("ask")} disabled={!draft.trim()}>{t.home.ask}</Button>
            </div>
          </div>
        ) : null}
        <div className="home-feed">
          {shown.length === 0 ? <p className="muted">{t.home.empty}</p> : null}
          {shown.map((m) => (
            <FeedCard key={`${m.space.id}/${m.entry.id}`} item={m} onOpen={() => open({ kind: "space", id: m.space.id })} />
          ))}
        </div>
      </section>

      <aside className="home-side">
        <section>
          <h2 className="list-head">{t.home.fleet}</h2>
          <ul className="fleet">
            {agents.map((a) => {
              const asks = cards.some((c) => c.agent === a.name);
              return (
                <li key={a.name}>
                  <button className="fleet-row" onClick={() => open({ kind: "agent", name: a.name })}>
                    <Dot state={a.active ? "working" : a.last_active ? "idle" : "never"} />
                    <span className="fleet-name">{a.name}</span>
                    <span className="fleet-status muted small">{a.active ? `${t.home.working}${a.provider ? ` · ${providerName(a.provider)}` : ""}` : a.last_active ? `${t.home.idle} · ${ago(a.last_active)}` : t.home.never}</span>
                    {asks ? <span className="fleet-kind rail-count">{t.cards.railTag}</span> : <span className="fleet-kind muted small">{kindTitle(a.image_name)}</span>}
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
        <section>
          <h2 className="list-head">{t.home.waitingOnYou}</h2>
          {cards.length === 0 && knocks.length === 0 ? <p className="muted small">{t.home.nothingWaiting}</p> : null}
          {cards.map((c) => (
            <button key={c.id} className="waiting" onClick={() => open({ kind: "agent", name: c.agent })}>
              <span className="waiting-who">{c.agent}</span>
              <span className="waiting-what">{c.question}</span>
            </button>
          ))}
          {knocks.map((k) => (
            <div key={k.knock_id} className="waiting waiting-knock">
              <span className="waiting-who mono">{k.address}</span>
              <span className="waiting-what">{k.authorization} → {k.space_title}</span>
              <span className="row-actions">
                <Button small kind="primary" onClick={() => answerKnock(k.knock_id, true)}>{t.spaces.letIn}</Button>
                <Button small kind="quiet" onClick={() => answerKnock(k.knock_id, false)}>{t.spaces.notNow}</Button>
              </span>
            </div>
          ))}
        </section>
        <section>
          <h2 className="list-head">{t.home.doneToday}</h2>
          {doneToday.length === 0 ? <p className="muted small">{t.home.empty}</p> : null}
          <ul className="done-list">
            {doneToday.map((m) => (
              <li key={m.entry.id} className="small"><strong>{m.entry.author}</strong> · {m.entry.title}</li>
            ))}
          </ul>
        </section>
      </aside>
    </div>
  );
}

function FeedCard({ item, onOpen }: { item: HomeMove; onOpen: () => void }) {
  const { entry: m, space } = item;
  const kind = t.spaces.moveKinds[m.kind] ?? m.kind;
  const state = t.spaces.states[m.state] ?? "";
  const f = m.fields as Record<string, unknown>;
  const who = m.author === space.joined_as ? t.spaces.you : m.author;
  return (
    <article className={`feed-card move-${m.kind}`}>
      <header className="move-head">
        {kind ? <span className="move-kind">{kind}</span> : null}
        <span className="move-who">{who}</span>
        <span className="muted small">{t.home.in}</span>
        <button className="space-chip" onClick={onOpen}>{space.title}</button>
        <span className="muted small">{time(m.at)}</span>
        {state ? <Chip tone={m.state === "done" || m.state === "issued" ? "ok" : m.state === "open" ? "accent" : "plain"}>{state}</Chip> : null}
      </header>
      {m.title ? <h3 className="move-title selectable">{m.title}</h3> : null}
      {m.body ? <div className="move-body feed-body"><Markdown text={m.body} /></div> : null}
      {m.kind === "ask" && (f.needs || f.ceiling) ? <div className="muted small">{[f.needs ? `${t.spaces.fields.needs}: ${f.needs}` : null, f.ceiling ? `${t.spaces.fields.ceiling}: ${f.ceiling}` : null].filter(Boolean).join(" · ")}</div> : null}
      {m.kind === "run" && f.measured ? <div className="muted small">{t.spaces.fields.measured}: {String(f.measured)}</div> : null}
    </article>
  );
}

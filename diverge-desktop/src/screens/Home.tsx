import { cardLine } from "../lib/cards";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { HomeMove } from "../bindings/HomeMove";
import { Markdown } from "../components/Markdown";
import { Button, Chip, Dot } from "../components/ui";
import { useShared } from "../lib/context";
import { ago, kindTitle, providerName, time } from "../lib/format";
import { api } from "../lib/ipc";
import { KnockCard } from "../components/Knock";
import { t } from "../strings";

const VIEWS: Record<string, (m: HomeMove) => boolean> = {
  all: (m) => !m.entry.parent && !["say", "admitted", "removed", "charter", "doorway"].includes(m.entry.kind) && m.space.kind !== "dm",
  asks: (m) => m.entry.kind === "ask",
  shows: (m) => m.entry.kind === "show",
  board: (m) => ["task", "offering"].includes(m.entry.kind) && !m.entry.parent,
  runs: (m) => m.entry.kind === "run",
  ideas: (m) => ["direction", "synthesis"].includes(m.entry.kind),
};

export function Home() {
  const { agents, open, cards, knocks, answerKnock, homeId, spaces } = useShared();
  const [feed, setFeed] = useState<HomeMove[]>([]);
  const [view, setView] = useState("all");
  const [draft, setDraft] = useState("");
  const [asking, setAsking] = useState(false);
  const [askTo, setAskTo] = useState<Set<string>>(new Set());
  const [sent, setSent] = useState<string | null>(null);
  const askable = spaces.filter((x) => ["home", "board", "idea"].includes(x.kind) && x.online);

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

  const startAsk = () => {
    setAsking(true);
    setSent(null);
    setAskTo(new Set(homeId ? [homeId] : []));
  };

  const sendAsk = async () => {
    const text = draft.trim();
    if (!text || askTo.size === 0) return;
    const out = await api.asksSend(text, null, null, [...askTo]);
    const ok = out.filter((o) => o.outcome.outcome === "ok").length;
    setSent(`${t.home.askSent} ${ok} ${ok === 1 ? t.home.room : t.home.rooms}.`);
    setDraft("");
    setAsking(false);
    setView("asks");
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
              <Button small kind="plain" onClick={startAsk} disabled={!draft.trim()}>{t.home.ask}</Button>
            </div>
            {asking ? (
              <div className="ask-to">
                <span className="muted small">{t.home.askWhere}</span>
                {askable.map((x) => (
                  <label key={x.id} className="switch">
                    <input type="checkbox" checked={askTo.has(x.id)} onChange={(e) => setAskTo((cur) => { const next = new Set(cur); if (e.target.checked) next.add(x.id); else next.delete(x.id); return next; })} />
                    <span>{x.title}</span>
                  </label>
                ))}
                <Button small kind="primary" onClick={sendAsk} disabled={askTo.size === 0 || !draft.trim()}>{t.home.askSend} {askTo.size} {askTo.size === 1 ? t.home.room : t.home.rooms}</Button>
                <Button small kind="quiet" onClick={() => setAsking(false)}>{t.spaces.cancel}</Button>
              </div>
            ) : null}
            {sent ? <p className="ok small">{sent}</p> : null}
          </div>
        ) : null}
        <div className="home-feed">
          {view === "asks" ? (
            <AskThreads feed={feed} askable={askable} onOpen={(id) => open({ kind: "space", id })} />
          ) : (
            <>
              {shown.length === 0 ? <p className="muted">{t.home.empty}</p> : null}
              {shown.map((m) => (
                <FeedCard key={`${m.space.id}/${m.entry.id}`} item={m} onOpen={() => open({ kind: "space", id: m.space.id })} />
              ))}
            </>
          )}
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
              <span className="waiting-what">{cardLine(c)}</span>
            </button>
          ))}
          {knocks.map((k) => (
            <KnockCard key={k.knock_id} knock={k} compact onAnswer={(yes) => answerKnock(k.knock_id, yes)} />
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
  const who = m.by === space.you_key ? t.spaces.you : m.agent_of ? `${m.author} (${m.agent_of === space.you_are ? t.spaces.yourAgent : `${t.spaces.runBy} ${m.agent_of}`})` : m.author;
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

type Place = { space: HomeMove["space"]; ask: HomeMove["entry"]; offers: HomeMove[]; replies: HomeMove[] };
type Thread = { key: string; what: string; mine: boolean; at: string; by: string; places: Place[] };

/** Your asks, each followed as one thread across every room it went to; and asks you could help with. */
function AskThreads(props: { feed: HomeMove[]; askable: HomeMove["space"][]; onOpen: (id: string) => void }) {
  const threads = useMemo(() => {
    const byKey = new Map<string, Thread>();
    for (const m of props.feed.filter((x) => x.entry.kind === "ask")) {
      const f = m.entry.fields as Record<string, unknown>;
      const key = typeof f.thread === "string" ? f.thread : `${m.space.id}/${m.entry.id}`;
      const children = props.feed.filter((x) => x.space.id === m.space.id && x.entry.parent === m.entry.id);
      const place: Place = { space: m.space, ask: m.entry, offers: children.filter((x) => x.entry.kind === "offer"), replies: children.filter((x) => x.entry.kind === "reply") };
      const th = byKey.get(key) ?? { key, what: m.entry.title, mine: m.entry.by === m.space.you_key, at: m.entry.at, by: m.entry.author, places: [] };
      th.places.push(place);
      if (m.entry.at > th.at) th.at = m.entry.at;
      byKey.set(key, th);
    }
    return [...byKey.values()].sort((a, b) => b.at.localeCompare(a.at));
  }, [props.feed]);
  const mine = threads.filter((x) => x.mine);
  const theirs = threads.filter((x) => !x.mine);
  const count = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
  return (
    <>
      <h2 className="list-head">{t.home.yourAsks}</h2>
      {mine.length === 0 ? <p className="muted small">{t.home.noAsks}</p> : null}
      {mine.map((th) => {
        const answered = th.places.some((p) => p.offers.length || p.replies.length);
        const elsewhere = props.askable.filter((r) => !th.places.some((p) => p.space.id === r.id));
        return (
          <article key={th.key} className="feed-card ask-thread">
            <header className="move-head">
              <span className="move-kind">{t.spaces.moveKinds.ask}</span>
              <span className="muted small">{time(th.at)}</span>
              <Chip tone={answered ? "ok" : "plain"}>{answered ? t.home.answered : t.home.noAnswerYet}</Chip>
            </header>
            <h3 className="move-title">{th.what}</h3>
            <ul className="ask-places">
              {th.places.map((p) => (
                <li key={p.space.id}>
                  <span>
                    <button className="space-chip" onClick={() => props.onOpen(p.space.id)}>{p.space.title}</button>{" "}
                    <span className="muted small">
                      {[p.offers.length ? count(p.offers.length, t.home.offer, t.home.offersWord) : null, p.replies.length ? count(p.replies.length, t.home.reply, t.home.replies) : null].filter(Boolean).join(" · ") || t.home.nothingYet}
                    </span>
                  </span>
                  {p.offers.map((o) => (
                    <div key={o.entry.id} className="ask-offer small">
                      <strong>{o.entry.author}</strong>{o.entry.agent_of ? ` (${t.spaces.runBy} ${o.entry.agent_of})` : ""}: {o.entry.body}
                    </div>
                  ))}
                </li>
              ))}
            </ul>
            {!answered && elsewhere.length ? <p className="muted small">{t.home.alsoAsk} {elsewhere.map((r) => r.title).join(", ")}.</p> : null}
          </article>
        );
      })}
      <h2 className="list-head">{t.home.theirAsks}</h2>
      {theirs.length === 0 ? <p className="muted small">{t.home.noTheirAsks}</p> : null}
      {theirs.map((th) => (
        <article key={th.key} className="feed-card">
          <header className="move-head">
            <span className="move-kind">{t.spaces.moveKinds.ask}</span>
            <span className="move-who">{th.by}</span>
            <span className="muted small">{time(th.at)}</span>
          </header>
          <h3 className="move-title">{th.what}</h3>
          <div className="row-actions">
            {th.places.map((p) => <button key={p.space.id} className="space-chip" onClick={() => props.onOpen(p.space.id)}>{p.space.title}</button>)}
          </div>
        </article>
      ))}
    </>
  );
}

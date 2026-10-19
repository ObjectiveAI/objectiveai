import { cardLine } from "../lib/cards";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { MoveView } from "../bindings/MoveView";
import type { PersonView } from "../bindings/PersonView";
import { Markdown } from "../components/Markdown";
import { Button, Card, Chip, Dot, Empty, Row, SectionHead } from "../components/ui";
import { useShared } from "../lib/context";
import { ago, kindTitle, time, dmWith } from "../lib/format";
import { api } from "../lib/ipc";
import { Conversation } from "./Conversation";
import { KnockCard } from "../components/Knock";
import { t } from "../strings";

type Thread = { key: string; kind: "agent" | "dm"; name: string; id: string; at: string | null; sub: string };

export function Inbox() {
  const { agents, spaces, cards, knocks, refreshSpaces, answerKnock } = useShared();
  const [people, setPeople] = useState<PersonView[]>([]);
  const [dmLast, setDmLast] = useState<Record<string, MoveView | undefined>>({});
  const [selected, setSelected] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);

  const load = useCallback(async () => {
    setPeople(await api.people());
    const last: Record<string, MoveView | undefined> = {};
    for (const s of spaces.filter((x) => x.kind === "dm")) {
      const feed = await api.spaceFeed(s.id);
      if (feed.outcome === "feed") last[s.id] = feed.moves[feed.moves.length - 1];
    }
    setDmLast(last);
  }, [spaces]);
  useEffect(() => {
    load();
  }, [load]);

  const threads: Thread[] = useMemo(() => {
    const list: Thread[] = [
      ...agents.map((a) => ({ key: `agent:${a.name}`, kind: "agent" as const, name: a.name, id: a.name, at: a.last_active, sub: kindTitle(a.image_name) })),
      ...spaces.filter((s) => s.kind === "dm").map((s) => ({ key: `dm:${s.id}`, kind: "dm" as const, name: dmWith(s), id: s.id, at: dmLast[s.id]?.at ?? null, sub: dmLast[s.id]?.body ?? "" })),
    ];
    return list.sort((a, b) => (b.at ?? "").localeCompare(a.at ?? ""));
  }, [agents, spaces, dmLast]);

  const startDm = async (p: PersonView) => {
    const existing = spaces.find((s) => s.kind === "dm" && dmWith(s) === p.name);
    if (existing) {
      setSelected(`dm:${existing.id}`);
      setPicking(false);
      return;
    }
    const out = await api.spaceHost({ title: p.name, kind: "dm", charter: "Two people. Nothing leaves this room.", open_door: false });
    if (out.outcome === "hosted") {
      // The room lets in the key you know them by, in the rooms you share.
      await api.spaceCall(out.id, "admit", { key: p.key, name: p.name });
      await refreshSpaces();
      setSelected(`dm:${out.id}`);
    }
    setPicking(false);
  };

  const current = threads.find((x) => x.key === selected);
  const waiting = cards.length + knocks.length;

  return (
    <div className="inbox">
      <aside className="inbox-list">
        <header className="pane-head">
          <h1>{t.inbox.title}</h1>
          <p className="muted small">{t.inbox.note}{waiting ? ` · ${waiting} ${t.home.waitingOnYou.toLowerCase()}` : ""}</p>
        </header>
        <div className="inbox-new">
          <Button small kind="tertiary" onClick={() => setPicking((p) => !p)}>+ {t.inbox.newDm}</Button>
          {picking ? (
            <div className="inbox-pick">
              <span className="muted small">{t.inbox.pick}</span>
              {people.filter((p) => !p.is_agent).map((p) => (
                <Row key={p.name} className="list-row" onClick={() => startDm(p)}><span>{p.name}</span><span className="muted small">{p.spaces.length} {t.spaces.title.toLowerCase()}</span></Row>
              ))}
            </div>
          ) : null}
        </div>
        {waiting ? (
          <div className="inbox-waiting">
            <SectionHead small level={3} title={t.home.waitingOnYou} />
            {cards.map((c) => (
              <Card key={c.id} className="waiting" onClick={() => setSelected(`agent:${c.agent}`)}>
                <span className="waiting-who">{c.agent}</span>
                <span className="waiting-what">{cardLine(c)}</span>
              </Card>
            ))}
            {knocks.map((k) => (
              <KnockCard key={k.knock_id} knock={k} compact onAnswer={(yes) => answerKnock(k.knock_id, yes)} />
            ))}
          </div>
        ) : null}
        <ul className="threads">
          {threads.length === 0 ? <li className="muted small">{t.inbox.noThreads}</li> : null}
          {threads.map((th) => {
            const agent = th.kind === "agent" ? agents.find((a) => a.name === th.name) : undefined;
            const asks = th.kind === "agent" && cards.some((c) => c.agent === th.name);
            return (
              <li key={th.key}>
                <Row className="thread" on={selected === th.key} onClick={() => setSelected(th.key)}>
                  <Dot state={agent ? (agent.active ? "working" : agent.last_active ? "idle" : "never") : "idle"} />
                  <span className="thread-name">{th.name}</span>
                  {asks ? <span className="rail-count">{t.cards.railTag}</span> : th.kind === "agent" ? <Chip>{t.spaces.agent}</Chip> : null}
                  <span className="thread-sub muted small">{th.sub}</span>
                  <span className="thread-at muted small">{th.at ? ago(th.at) : ""}</span>
                </Row>
              </li>
            );
          })}
        </ul>
      </aside>
      <section className="inbox-thread">
        {!current ? <Empty title={t.inbox.empty} /> : current.kind === "agent" ? <Conversation key={current.id} name={current.id} tabKey={current.key} /> : <DmThread key={current.id} id={current.id} title={current.name} onChanged={load} />}
      </section>
    </div>
  );
}

function DmThread({ id, title, onChanged }: { id: string; title: string; onChanged: () => void }) {
  const [moves, setMoves] = useState<MoveView[]>([]);
  const [draft, setDraft] = useState("");
  const load = useCallback(async () => {
    const feed = await api.spaceFeed(id);
    if (feed.outcome === "feed") setMoves(feed.moves);
  }, [id]);
  useEffect(() => {
    load();
    let scope: string | null = null;
    let closed = false;
    api.spaceWatch(id, (e) => { if (e.event === "updated") { load(); onChanged(); } }).then((s) => { scope = s; if (closed) api.scopeClose(s); });
    return () => { closed = true; if (scope) api.scopeClose(scope); };
  }, [id, load, onChanged]);
  const send = async () => {
    const text = draft.trim();
    if (!text) return;
    setDraft("");
    await api.spaceCall(id, "say", { body: text });
    load();
  };
  return (
    <div className="convo">
      <header className="convo-head"><div className="convo-title"><h1>{title}</h1><span className="muted small">{t.spaces.kinds.dm}</span></div></header>
      <div className="convo-scroll">
        <div className="convo-body">
          {moves.length === 0 ? <p className="muted convo-empty">{t.inbox.nothingSaid}</p> : null}
          {moves.map((m) => (
            <div key={m.id} className={`msg ${m.author === "me" ? "msg-user" : "msg-agent"}`}>
              <div className="msg-meta"><span>{m.author === "me" ? t.convo.you : m.author}</span><span className="muted">{time(m.at)}</span></div>
              <div className={m.author === "me" ? "bubble" : ""}><Markdown text={m.body || m.title} /></div>
            </div>
          ))}
        </div>
      </div>
      <footer className="composer">
        <div className="composer-row">
          <textarea rows={2} value={draft} placeholder={`${t.inbox.placeholder} ${title}`} onChange={(e) => setDraft(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); } }} />
          <Button kind="primary" onClick={send} disabled={!draft.trim()}>{t.inbox.send}</Button>
        </div>
      </footer>
    </div>
  );
}

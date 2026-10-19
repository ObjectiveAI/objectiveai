import { answerLabel, cardDetails, cardLine, cardNote, inTimeOrder } from "../lib/cards";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CardView } from "../bindings/CardView";
import type { LogEntry } from "../bindings/LogEntry";
import { Markdown } from "../components/Markdown";
import { MountsPanel } from "../components/Mounts";
import { Button, Card, Chip, Dot, Escapes } from "../components/ui";
import { useShared } from "../lib/context";
import { argsLine, fold, pretty, shown, type Block, type Part, type WorkSummary } from "../lib/conversation";
import { ago, kindTitle, number, providerName, time } from "../lib/format";
import { api, errorText, ticket as newTicket } from "../lib/ipc";
import { t } from "../strings";

type Pending = { ticket: string; text: string; state: "waiting" | "taken" | "late" | "error"; error?: string };

export function Conversation(props: { name: string; tabKey: string }) {
  const { agents, refreshAgents, cards, withdrawn, answerCard } = useShared();
  const mine = cards.filter((c) => c.agent === props.name);
  const gone = withdrawn.filter((c) => c.agent === props.name);
  const agent = agents.find((a) => a.name === props.name);
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [watching, setWatching] = useState(true);
  const [problem, setProblem] = useState<string | null>(null);
  const [pending, setPending] = useState<Pending[]>([]);
  const [draft, setDraft] = useState("");
  const [removing, setRemoving] = useState<"idle" | "confirm" | "refused">("idle");
  const [mountsOpen, setMountsOpen] = useState(false);
  const scroller = useRef<HTMLDivElement>(null);
  const stick = useRef(true);
  const lastIndex = useRef(0);

  // Read the whole log, then keep watching what lands.
  useEffect(() => {
    if (!watching) return;
    let scope: string | null = null;
    let closed = false;
    api
      .logsOpen({ name: props.name, logs_index_from: lastIndex.current + 1, logs_index_to: null, created_from: null, created_to: null, item_type: null, jq: null, count: null, watch: true }, (event) => {
        if (event.event === "entry") {
          const entry = event.entry;
          if (entry.logs_index <= lastIndex.current) return;
          lastIndex.current = entry.logs_index;
          setEntries((list) => [...list, entry]);
        } else if (event.event === "error") setProblem(event.message);
      })
      .then((id) => {
        scope = id;
        if (closed) api.scopeClose(id);
      })
      .catch((e) => setProblem(errorText(e)));
    return () => {
      closed = true;
      if (scope) api.scopeClose(scope);
    };
  }, [props.name, watching]);

  const blocks = useMemo(() => fold(entries), [entries]);
  // Whether it is running, read from its own log — live, unlike the list.
  const runningOn = useMemo(() => {
    for (let i = blocks.length - 1; i >= 0; i--) {
      const b = blocks[i];
      if (b.kind === "started") return b.provider;
      if (b.kind === "finished") return null;
    }
    return agent?.active ? agent.provider : null;
  }, [blocks, agent?.active, agent?.provider]);
  const running = runningOn !== null;
  const lastFinished = useMemo(() => {
    for (let i = blocks.length - 1; i >= 0; i--) {
      const b = blocks[i];
      if (b.kind === "finished") return b.at;
    }
    return agent?.last_active ?? null;
  }, [blocks, agent?.last_active]);

  useEffect(() => {
    const el = scroller.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
  }, [blocks, pending]);

  const send = useCallback(async () => {
    const text = draft.trim();
    if (!text) return;
    const ticket = newTicket();
    setDraft("");
    setPending((p) => [...p, { ticket, text, state: "waiting" }]);
    stick.current = true;
    try {
      const outcome = await api.agentsMessage(props.name, text, ticket);
      setPending((p) =>
        outcome.outcome === "delivered"
          ? p.filter((x) => x.ticket !== ticket)
          : p.map((x) => (x.ticket === ticket ? { ...x, state: outcome.outcome === "cancelled" ? "taken" : "error", error: outcome.outcome === "error" ? outcome.message : undefined } : x)),
      );
      if (outcome.outcome === "cancelled") setTimeout(() => setPending((p) => p.filter((x) => x.ticket !== ticket)), 2500);
    } catch (e) {
      setPending((p) => p.map((x) => (x.ticket === ticket ? { ...x, state: "error", error: errorText(e) } : x)));
    }
    refreshAgents();
  }, [draft, props.name, refreshAgents]);

  const takeBack = async (ticket: string) => {
    const had = await api.takeBack(ticket);
    if (!had) setPending((p) => p.map((x) => (x.ticket === ticket ? { ...x, state: "late" } : x)));
  };

  const remove = async () => {
    const outcome = await api.agentsDelete(props.name);
    if (outcome.outcome === "active") setRemoving("refused");
    else if (outcome.outcome === "error") setProblem(outcome.message);
    else setRemoving("idle");
    refreshAgents();
  };

  const status = runningOn ? (
    <Chip tone="ok">
      <Dot state="working" /> {t.status.working} {t.status.on} {providerName(runningOn)}
    </Chip>
  ) : lastFinished ? (
    <Chip>
      <Dot state="idle" /> {t.status.idle} · {ago(lastFinished)}
    </Chip>
  ) : (
    <Chip>
      <Dot state="never" /> {t.status.never}
    </Chip>
  );

  return (
    <div className="convo">
      <header className="convo-head">
        <div className="convo-title">
          <h1>{props.name}</h1>
          <span className="muted">{agent ? kindTitle(agent.image_name) : ""}</span>
          {status}
        </div>
        <div className="convo-actions">
          <Button small kind="tertiary" onClick={() => setMountsOpen((o) => !o)}>{t.mounts.button}</Button>
          <Button small kind="tertiary" onClick={() => setWatching((w) => !w)} title={watching ? t.convo.watching : t.convo.notWatching}>
            {watching ? t.convo.stopWatching : t.convo.watch}
          </Button>
          {removing === "confirm" ? (
            <Escapes className="confirm-pair" onEscape={() => setRemoving("idle")}>
              <Button small kind="danger" onClick={remove}>{t.convo.removeConfirm}</Button>
              <Button small kind="tertiary" onClick={() => setRemoving("idle")}>{t.convo.removeCancel}</Button>
            </Escapes>
          ) : (
            <Button small kind="tertiary" onClick={() => setRemoving("confirm")}>{t.convo.remove}</Button>
          )}
        </div>
      </header>
      {removing === "refused" ? (
        <div className="banner banner-warn">
          {t.convo.removeActive} <Button small kind="tertiary" onClick={() => setRemoving("idle")}>{t.common.ok}</Button>
        </div>
      ) : null}
      {problem ? <div className="banner banner-bad">{problem}</div> : null}
      {mountsOpen ? <MountsPanel name={props.name} active={!!agent?.active} onClose={() => setMountsOpen(false)} /> : null}

      <div
        className="convo-scroll"
        ref={scroller}
        onScroll={(e) => {
          const el = e.currentTarget;
          stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
        }}
      >
        <div className="convo-body">
          {blocks.length === 0 ? <p className="muted convo-empty">{t.convo.empty}</p> : null}
          {inTimeOrder(blocks, gone).map((placed) =>
            "card" in placed ? (
              <WithdrawnCard key={`withdrawn-${placed.card.id}`} card={placed.card} />
            ) : (
              <BlockView key={placed.index} block={placed.item} agentName={props.name} live={running && placed.index === blocks.length - 1} />
            ),
          )}
          {mine.map((card) => (
            <AskCard key={card.id} card={card} onAnswer={(a) => answerCard(card.id, a)} />
          ))}
          {running && mine.length === 0 ? (
            <div className="working">
              <span className="working-dots"><i /><i /><i /></span> {t.convo.working}
            </div>
          ) : null}
        </div>
      </div>

      <footer className="composer">
        {pending.length > 0 ? (
          <ul className="pending">
            {pending.map((p) => (
              <li key={p.ticket} className={`pending-item pending-${p.state}`}>
                <span className="pending-text">{p.text}</span>
                {p.state === "waiting" ? (
                  <>
                    <span className="muted">{t.convo.waiting}</span>
                    <Button small kind="tertiary" onClick={() => takeBack(p.ticket)}>{t.convo.takeBack}</Button>
                  </>
                ) : p.state === "taken" ? (
                  <span className="muted">{t.convo.takenBack}</span>
                ) : p.state === "late" ? (
                  <span className="muted">{t.convo.tooLate}</span>
                ) : (
                  <span className="bad">{p.error}</span>
                )}
              </li>
            ))}
          </ul>
        ) : null}
        {running ? <p className="composer-note muted" title={t.convo.noStop}>{t.convo.queueNote}</p> : null}
        <div className="composer-row">
          <textarea
            rows={2}
            value={draft}
            placeholder={`${t.convo.placeholder} ${props.name}`}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                send();
              }
            }}
          />
          <Button kind="primary" onClick={send} disabled={!draft.trim()}>{t.convo.send}</Button>
        </div>
      </footer>
    </div>
  );
}

/** A card its agent stopped waiting on: what it asked, said to be withdrawn, with nothing to answer. */
function WithdrawnCard({ card }: { card: CardView }) {
  return (
    <Card tone="quiet">
      <div className="ask-card-head">
        <strong>{card.from ?? card.agent}</strong> <span>{t.cards.stoppedWaiting}</span>
        <Chip>{t.cards.withdrawn}</Chip>
      </div>
      <p className="selectable">{cardLine(card)}</p>
      <p className="muted small">{t.cards.withdrawnNote}</p>
    </Card>
  );
}

function AskCard({ card, onAnswer }: { card: CardView; onAnswer: (answer: string) => void }) {
  const [text, setText] = useState("");
  return (
    <div className={`ask-card ask-card-${card.kind}`}>
      <div className="ask-card-head">
        {card.from ? (
          <>
            <strong>{card.from}</strong> <span>{t.cards.asksOf} {card.agent}</span>
          </>
        ) : (
          <>
            <strong>{card.agent}</strong> <span>{t.cards.asksYou}</span>
          </>
        )}
        <Chip tone="accent">{t.cards.kinds[card.kind]}</Chip>
        <span className="muted small">{card.from ? t.cards.waitingFor : t.cards.waiting}</span>
      </div>
      <p className="ask-card-question selectable">{cardLine(card)}</p>
      {cardDetails(card).length > 0 ? (
        <dl className="ask-card-details">
          {cardDetails(card).map(([k, v]) => (
            <div key={k}>
              <dt>{k}</dt>
              <dd className="selectable">{v}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {cardNote(card) ? <p className="muted small">{cardNote(card)}</p> : null}
      {card.kind === "credential" ? <p className="muted small">{t.cards.credentialNote}</p> : null}
      {card.kind === "question" ? (
        <div className="composer-row">
          <input value={text} placeholder={t.cards.placeholder} onChange={(e) => setText(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter" && text.trim()) onAnswer(text.trim()); }} />
          <Button kind="primary" onClick={() => onAnswer(text.trim())} disabled={!text.trim()}>{t.cards.send}</Button>
        </div>
      ) : (
        <div className="ask-card-options">
          {card.options.map((o) => (
            <Button key={o} kind={card.kind === "credential" ? "secondary" : o === "no" || o === "decline" ? "tertiary" : "primary"} onClick={() => onAnswer(o)}>{answerLabel(o)}</Button>
          ))}
          {card.kind === "credential" ? <Button kind="tertiary" onClick={() => onAnswer("")}>{t.cards.none}</Button> : null}
        </div>
      )}
    </div>
  );
}

function BlockView({ block, agentName, live }: { block: Block; agentName: string; live: boolean }) {
  switch (block.kind) {
    case "user":
      return (
        <div className="msg msg-user">
          <div className="msg-meta"><span>{t.convo.you}</span><span className="muted">{time(block.at)}</span></div>
          <div className="bubble"><Markdown text={block.text} />{block.attachments.map((a, i) => <Chip key={i}>{a}</Chip>)}</div>
        </div>
      );
    case "turn":
      return (
        <div className="msg msg-agent">
          <div className="msg-meta"><span>{agentName}</span><span className="muted">{time(block.at)}</span></div>
          <Turn parts={block.parts} live={live} />
          {block.usage ? (
            <div className="usage muted" title={t.standIn.note}>
              {t.convo.measured}: {number(block.usage.total)} {t.convo.tokens} ({number(block.usage.prompt)} in · {number(block.usage.completion)} out)
            </div>
          ) : null}
        </div>
      );
    case "started":
      return <div className="marker"><span>{t.convo.started} {t.status.on} {providerName(block.provider)} · {time(block.at)}</span></div>;
    case "finished":
      return <div className="marker"><span>{t.convo.finished} · {time(block.at)}</span></div>;
    case "error":
      return <Card tone="bad" className="selectable"><strong>{t.common.error}</strong><p>{block.message}</p></Card>;
    case "notice":
      return (
        <Card tone={block.fatal ? "bad" : "quiet"} className="selectable">
          <strong>{block.fatal ? t.convo.fatal : t.convo.notice}</strong>
          <p className="mono">{typeof block.message === "string" ? block.message : JSON.stringify(block.message)}</p>
        </Card>
      );
  }
}

function duration(seconds: number): string {
  return seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
}

/** What the agent said, prominent; the work between, one line each. */
function Turn({ parts, live }: { parts: Part[]; live: boolean }) {
  const view = shown(parts);
  return (
    <div className="parts">
      {view.map((v, i) =>
        v.kind === "say" ? <Parts key={i} parts={[v.part]} /> : <Work key={i} steps={v.steps} summary={v.summary} live={live && i === view.length - 1} />,
      )}
    </div>
  );
}

function Work({ steps, summary, live }: { steps: Part[]; summary: WorkSummary; live: boolean }) {
  const working = live && (summary.waiting || !summary.last || !!summary.last.answer);
  const bits = [
    `${working ? t.convo.workingNow : t.convo.worked}${summary.seconds ? ` ${duration(summary.seconds)}` : ""}`,
    ...summary.tools.map((x) => `${t.convo.doorTools[x.name] ?? x.name}${x.count > 1 ? ` ×${x.count}` : ""}`),
    summary.thought ? t.convo.thought : null,
    summary.helpers ? `${summary.helpers} ${summary.helpers > 1 ? t.convo.helpers : t.convo.helperOne}` : null,
  ].filter(Boolean);
  const now = live && summary.last && !summary.last.answer ? `${t.convo.doorTools[summary.last.name] ?? summary.last.name}: ${argsLine(summary.last.arguments)}` : null;
  return (
    <details className={`work${working ? " live" : ""}${summary.failed ? " has-failed" : ""}`}>
      <summary>
        {working ? <span className="working-dots small"><i /><i /><i /></span> : null}
        <span className="work-line">{bits.join(" · ")}</span>
        {summary.failed ? <span className="work-failed">{summary.failed} {t.convo.failed}</span> : null}
        {now ? <span className="work-now mono">{now}</span> : null}
      </summary>
      <div className="work-steps">
        <Parts parts={steps} flat />
      </div>
    </details>
  );
}

function Parts({ parts, flat }: { parts: Part[]; flat?: boolean }) {
  return (
    <div className="parts">
      {parts.map((part, i) => {
        switch (part.kind) {
          case "text":
            return <Markdown key={i} text={part.text} />;
          case "reasoning":
            if (flat) return <p key={i} className="thought-text selectable">{part.text}</p>;
            return (
              <details key={i} className="thinking">
                <summary>{t.convo.thinking}</summary>
                <p className="selectable">{part.text}</p>
              </details>
            );
          case "refusal":
            return <Card key={i} tone="warn"><strong>{t.convo.refusal}</strong><p>{part.text}</p></Card>;
          case "image":
            return <img key={i} className="agent-image" alt="" src={`data:${part.mime};base64,${part.data}`} />;
          case "audio":
            return <Chip key={i}>{t.convo.audio}</Chip>;
          case "tool":
            return <Tool key={i} part={part} />;
        }
      })}
    </div>
  );
}

function Tool({ part }: { part: Extract<Part, { kind: "tool" }> }) {
  const state = part.answer ? (part.answer.isError ? "bad" : "done") : "waiting";
  return (
    <details className={`tool tool-${state}`}>
      <summary>
        <span className="tool-name mono">{part.name}</span>
        <span className="tool-args mono">{argsLine(part.arguments)}</span>
        <span className="tool-state">
          {state === "waiting" ? <span className="working-dots small"><i /><i /><i /></span> : state === "bad" ? t.convo.failed : null}
        </span>
      </summary>
      <div className="tool-body">
        {part.arguments ? (
          <>
            <span className="tool-label">{t.convo.arguments}</span>
            <pre className="selectable">{pretty(part.arguments)}</pre>
          </>
        ) : null}
        {part.parts.length > 0 ? (
          <div className="helper">
            <span className="tool-label">{t.convo.helper}</span>
            <Turn parts={part.parts} live={!part.answer} />
          </div>
        ) : null}
        {part.answer ? (
          <>
            <span className="tool-label">{t.convo.answer}</span>
            <pre className={`selectable${part.answer.isError ? " bad" : ""}`}>{part.answer.text}</pre>
          </>
        ) : (
          <span className="muted">{t.convo.waitingForAnswer}</span>
        )}
      </div>
    </details>
  );
}

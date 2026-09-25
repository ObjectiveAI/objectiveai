import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { LogEntry } from "../bindings/LogEntry";
import { Markdown } from "../components/Markdown";
import { Button, Chip, Dot } from "../components/ui";
import { useShared } from "../lib/context";
import { argsLine, fold, pretty, type Block, type Part } from "../lib/conversation";
import { ago, kindTitle, number, providerName, time } from "../lib/format";
import { api, errorText, ticket as newTicket } from "../lib/ipc";
import { t } from "../strings";

type Pending = { ticket: string; text: string; state: "waiting" | "taken" | "late" | "error"; error?: string };

export function Conversation(props: { name: string; tabKey: string }) {
  const { agents, refreshAgents } = useShared();
  const agent = agents.find((a) => a.name === props.name);
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [watching, setWatching] = useState(true);
  const [problem, setProblem] = useState<string | null>(null);
  const [pending, setPending] = useState<Pending[]>([]);
  const [draft, setDraft] = useState("");
  const [removing, setRemoving] = useState<"idle" | "confirm" | "refused">("idle");
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
          <Button small kind="quiet" onClick={() => setWatching((w) => !w)} title={watching ? t.convo.watching : t.convo.notWatching}>
            {watching ? t.convo.stopWatching : t.convo.watch}
          </Button>
          {removing === "confirm" ? (
            <>
              <Button small kind="danger" onClick={remove}>{t.convo.removeConfirm}</Button>
              <Button small kind="quiet" onClick={() => setRemoving("idle")}>{t.convo.removeCancel}</Button>
            </>
          ) : (
            <Button small kind="quiet" onClick={() => setRemoving("confirm")}>{t.convo.remove}</Button>
          )}
        </div>
      </header>
      {removing === "refused" ? (
        <div className="banner banner-warn">
          {t.convo.removeActive} <button className="link" onClick={() => setRemoving("idle")}>{t.common.ok}</button>
        </div>
      ) : null}
      {problem ? <div className="banner banner-bad">{problem}</div> : null}

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
          {blocks.map((block, i) => (
            <BlockView key={i} block={block} agentName={props.name} />
          ))}
          {running ? (
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
                    <Button small kind="quiet" onClick={() => takeBack(p.ticket)}>{t.convo.takeBack}</Button>
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

function BlockView({ block, agentName }: { block: Block; agentName: string }) {
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
          <Parts parts={block.parts} />
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
      return <div className="card card-bad selectable"><strong>{t.common.error}</strong><p>{block.message}</p></div>;
    case "notice":
      return (
        <div className={`card ${block.fatal ? "card-bad" : "card-quiet"} selectable`}>
          <strong>{block.fatal ? t.convo.fatal : t.convo.notice}</strong>
          <p className="mono">{typeof block.message === "string" ? block.message : JSON.stringify(block.message)}</p>
        </div>
      );
  }
}

function Parts({ parts }: { parts: Part[] }) {
  return (
    <div className="parts">
      {parts.map((part, i) => {
        switch (part.kind) {
          case "text":
            return <Markdown key={i} text={part.text} />;
          case "reasoning":
            return (
              <details key={i} className="thinking">
                <summary>{t.convo.thinking}</summary>
                <p className="selectable">{part.text}</p>
              </details>
            );
          case "refusal":
            return <div key={i} className="card card-warn"><strong>{t.convo.refusal}</strong><p>{part.text}</p></div>;
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
            <Parts parts={part.parts} />
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

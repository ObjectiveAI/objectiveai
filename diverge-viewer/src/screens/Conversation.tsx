import { daemon } from "@diverge/sdk";
import { useMemo, useState } from "react";
import { Button, Card, Chip, Dot, Escapes } from "../components/ui";
import { errorText } from "../lib/app";
import { agentKey, useShared } from "../lib/context";
import { type AgentRef, type LogItem, type LogsRequest, type Turn, contentText, useLogs } from "../lib/conversation";
import { handle } from "../lib/daemon";
import { number, time } from "../lib/format";
import { type Identity, identityKey } from "../lib/machines";
import { t } from "../strings";
import { NotConnected } from "./NotConnected";

const agents = daemon.endpoints.agents;

/** A message sent and not yet taken by the agent: the daemon's pending answer, and the words it carries. */
type Waiting = { id: number; text: string; pending: daemon.client.pending.Pending<daemon.endpoints.agents.message.server.response.Frame>; state: "waiting" | "taking_back" };

export function Conversation(props: { template: string; index: number; name: string | null }) {
  const { connected, agents: listed, close } = useShared();
  const title = props.name ?? `${t.rail.unnamed} ${props.index}`;
  const ref = useMemo<AgentRef>(() => ({ template: props.template, index: props.index }), [props.template, props.index]);
  const request = useMemo<LogsRequest>(() => ({ agent: ref, watch: true }), [ref]);
  const log = useLogs(request, connected);
  const agent = listed.items.find((a) => a.template === props.template && a.index === props.index);
  if (!connected) return <NotConnected title={title} />;
  return (
    <div className="page convo">
      <div className="page-inner">
        <div className="head-row">
          <h1 className="page-title">{title}</h1>
          {agent ? <Dot state={agent.active ? "working" : agent.last_active ? "idle" : "never"} /> : null}
          <span className="mono muted">{props.template}</span>
          <Remove agent={ref} active={agent?.active ?? false} onRemoved={() => close(agentKey(ref))} />
        </div>
        {log.ended ? <Card tone="bad">{log.ended}</Card> : null}
        <div className="turns">
          {log.turns.length === 0 && !log.ended ? <p className="muted">{t.convo.empty}</p> : null}
          {log.turns.map((turn) => (
            <TurnView key={turn.item.logs_index} turn={turn} />
          ))}
        </div>
        <Composer agent={ref} />
      </div>
    </div>
  );
}

/** One log item as a turn, by its type. */
function TurnView({ turn }: { turn: Turn }) {
  const item = turn.item;
  const meta = (
    <span className="turn-meta muted">
      {time(item.created)} · {number(item.logs_index)}
    </span>
  );
  const quiet = (words: string, body?: string) => (
    <div className="turn turn-quiet">
      <span>{words}</span> {meta}
      {body ? <p className="pre">{body}</p> : null}
    </div>
  );
  switch (item.type) {
    case "user_text_content":
      return (
        <div className="turn turn-user">
          <strong>{t.convo.you}</strong> {meta}
          <p className="pre selectable">{item.text}</p>
        </div>
      );
    case "assistant_text_content":
      return (
        <div className="turn">
          <strong>{t.convo.agent}</strong> {meta}
          <p className="pre selectable">{item.text}</p>
        </div>
      );
    case "assistant_reasoning":
      return (
        <details className="more turn">
          <summary>
            {t.convo.thinking} {meta}
          </summary>
          <p className="pre selectable muted">{item.text}</p>
        </details>
      );
    case "assistant_refusal":
      return (
        <div className="turn">
          <Chip tone="warn">{t.convo.refusal}</Chip> {meta}
          <p className="pre selectable">{item.text}</p>
        </div>
      );
    case "assistant_tool_call":
      return (
        <details className="more turn">
          <summary>
            {t.convo.tool} <span className="mono">{item.name}</span> {meta}
          </summary>
          <p className="pre selectable mono">{item.arguments ?? ""}</p>
        </details>
      );
    case "tool_response":
      return (
        <details className="more turn">
          <summary>
            {t.convo.answer} <span className="mono">{item.id}</span> {item.isError ? <Chip tone="bad">{t.convo.failed}</Chip> : null} {meta}
          </summary>
          <p className="pre selectable">{contentText(item.content)}</p>
        </details>
      );
    case "usage":
      return quiet(`${t.convo.measured} ${number(item.total_tokens)} ${t.convo.tokens}`);
    case "notification":
      return (
        <Card tone={item.is_fatal ? "bad" : "warn"}>
          <strong>{item.is_fatal ? t.convo.fatal : t.convo.notice}</strong> {meta}
          <span className="pre selectable">{typeof item.message === "string" ? item.message : JSON.stringify(item.message)}</span>
        </Card>
      );
    case "error":
      return (
        <Card tone="bad">
          <strong>{t.common.error}</strong> {meta}
          <span className="pre selectable mono">{JSON.stringify(item.error)}</span>
        </Card>
      );
    case "active":
      return quiet(`${t.convo.started} ${t.status.on} ${where(item)}`);
    case "inactive":
      return quiet(`${t.convo.finished} ${t.status.on} ${where(item)}`);
    default:
      return quiet(t.itemTypes[item.type] ?? item.type);
  }
}

/** Where an agent ran, as its log names the provider. */
function where(item: LogItem & { identity: Identity }): string {
  return identityKey(item.identity);
}

/** The message field, and the messages sent that the agent has not yet taken. */
function Composer({ agent }: { agent: AgentRef }) {
  const [text, setText] = useState("");
  const [waiting, setWaiting] = useState<Waiting[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const settle = (id: number, outcome: string | null) => {
    setWaiting((w) => w.filter((m) => m.id !== id));
    if (outcome) setProblem(outcome);
  };
  const send = async () => {
    const words = text.trim();
    if (!words) return;
    setText("");
    setProblem(null);
    const id = Date.now() + Math.random();
    try {
      const pending = await agents.message.client.execute(handle, { agent, content: [{ type: "text", text: words }] });
      setWaiting((w) => [...w, { id, text: words, pending, state: "waiting" }]);
      const answer = await pending.wait();
      if (answer === "delivered" || answer === "cancelled") settle(id, null);
      else settle(id, answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
    } catch (error) {
      settle(id, errorText(error));
    }
  };
  const takeBack = (m: Waiting) => {
    setWaiting((w) => w.map((x) => (x.id === m.id ? { ...x, state: "taking_back" } : x)));
    m.pending.cancel().catch((error) => setProblem(errorText(error)));
  };
  return (
    <div className="composer">
      {waiting.map((m) => (
        <Card key={m.id} tone="quiet">
          <span className="pre">{m.text}</span>
          <span className="toolbar">
            <span className="muted">{t.convo.waiting}</span>
            <Button kind="tertiary" small onClick={() => takeBack(m)} disabled={m.state === "taking_back"}>
              {t.convo.takeBack}
            </Button>
          </span>
        </Card>
      ))}
      {problem ? <Card tone="bad">{problem}</Card> : null}
      <textarea
        rows={3}
        value={text}
        placeholder={t.convo.placeholder}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            void send();
          }
        }}
      />
      <div className="toolbar">
        <Button kind="primary" onClick={() => void send()} disabled={!text.trim()}>
          {t.convo.send}
        </Button>
        <span className="muted">{t.convo.queueNote}</span>
      </div>
    </div>
  );
}

/** Remove the agent: a confirm, then the daemon's one answer. */
function Remove({ agent, active, onRemoved }: { agent: AgentRef; active: boolean; onRemoved: () => void }) {
  const [asking, setAsking] = useState(false);
  const [said, setSaid] = useState<string | null>(null);
  const remove = async () => {
    setAsking(false);
    try {
      const answer = await agents.delete.client.execute(handle, { agent });
      if (answer === "deleted") onRemoved();
      else setSaid(answer === "active" ? t.convo.removeActive : answer === "not_found" ? t.convo.gone : answer === "forbidden" ? t.daemon.forbidden : JSON.stringify(answer));
    } catch (error) {
      setSaid(errorText(error));
    }
  };
  if (asking) {
    return (
      <Escapes onEscape={() => setAsking(false)} className="toolbar">
        <Button kind="danger" small onClick={() => void remove()}>
          {t.convo.removeConfirm}
        </Button>
        <Button kind="tertiary" small onClick={() => setAsking(false)}>
          {t.convo.removeCancel}
        </Button>
      </Escapes>
    );
  }
  return (
    <span className="toolbar">
      {said ? <span className="muted">{said}</span> : null}
      <Button kind="tertiary" small onClick={() => setAsking(true)} title={active ? t.convo.removeActive : undefined}>
        {t.convo.remove}
      </Button>
    </span>
  );
}

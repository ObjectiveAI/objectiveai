import { useCallback, useEffect, useMemo, useState } from "react";
import type { MoveView } from "../bindings/MoveView";
import type { SpaceView } from "../bindings/SpaceView";
import type { ToolView } from "../bindings/ToolView";
import { Door } from "../components/Door";
import { Markdown } from "../components/Markdown";
import { SchemaForm, initial, missing, type Schema } from "../components/SchemaForm";
import { Button, Chip, Dot, Empty } from "../components/ui";
import { useShared } from "../lib/context";
import { ago, providerName, time } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";

type Active = { tool: ToolView; values: Record<string, unknown> };

export function Space(props: { id: string; tabKey: string }) {
  const { refreshSpaces } = useShared();
  const [space, setSpace] = useState<SpaceView | null>(null);
  const [moves, setMoves] = useState<MoveView[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const [active, setActive] = useState<Active | null>(null);
  const [said, setSaid] = useState<string | null>(null);
  const [invite, setInvite] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const doorKey = `door-seen:${props.id}`;
  const [door, setDoor] = useState<boolean>(() => {
    try {
      return localStorage.getItem(doorKey) === null;
    } catch {
      return true;
    }
  });
  const stay = () => {
    try {
      localStorage.setItem(doorKey, "1");
    } catch {
      /* the door just shows again next time */
    }
    setDoor(false);
  };

  const load = useCallback(async () => {
    try {
      setSpace(await api.space(props.id));
      const feed = await api.spaceFeed(props.id);
      if (feed.outcome === "feed") setMoves(feed.moves);
      else setProblem(feed.message);
    } catch (e) {
      setProblem(errorText(e));
    }
  }, [props.id]);

  useEffect(() => {
    load();
    let scope: string | null = null;
    let closed = false;
    api.spaceWatch(props.id, (event) => {
      if (event.event === "updated") load();
    }).then((id) => {
      scope = id;
      if (closed) api.scopeClose(id);
    });
    return () => {
      closed = true;
      if (scope) api.scopeClose(scope);
    };
  }, [props.id, load]);

  const begin = (tool: ToolView, prefill: Record<string, unknown> = {}) => {
    const base = initial(tool.schema as Schema, tool.schema as Schema);
    setActive({ tool, values: { ...(base && typeof base === "object" ? (base as Record<string, unknown>) : {}), ...prefill } });
    setSaid(null);
  };

  const submit = async () => {
    if (!active) return;
    const out = await api.spaceCall(props.id, active.tool.name, active.values);
    if (out.outcome === "ok") {
      setSaid(`${t.spaces.did} ${out.text}`);
      setActive(null);
      load();
    } else setSaid(out.message);
  };

  const leave = async () => {
    try {
      await api.spaceLeave(props.id);
      refreshSpaces();
    } catch (e) {
      setProblem(errorText(e));
    }
  };

  const showInvite = async () => {
    const i = await api.spaceInvite(props.id);
    setInvite(i?.text ?? null);
    setCopied(false);
  };

  const byId = useMemo(() => new Map(moves.map((m) => [m.id, m])), [moves]);
  const roots = useMemo(() => [...moves].filter((m) => !m.parent || !byId.has(m.parent)).sort((a, b) => b.at.localeCompare(a.at)), [moves, byId]);
  const childrenOf = useCallback((id: string) => moves.filter((m) => m.parent === id).sort((a, b) => a.at.localeCompare(b.at)), [moves]);
  const tool = (name: string) => space?.tools.find((x) => x.name === name);
  const me = space?.summary.joined_as ?? "me";

  if (!space) return problem ? <div className="banner banner-bad">{problem}</div> : <Empty title={t.spaces.title} />;
  const s = space.summary;
  // The door: for a room someone else hosts, the first time you enter, and on request.
  if (door && !s.mine) return <Door space={space} onStay={stay} onLeave={leave} />;
  const incomplete = active ? missing(active.tool.schema as Schema, active.values) : [];

  return (
    <div className="space">
      <header className="space-head">
        <div className="space-title">
          <h1>{s.title}</h1>
          <Chip>{t.spaces.kinds[s.kind] ?? s.kind}</Chip>
          <span className="muted small">{s.mine ? t.spaces.youHostIt : `${t.spaces.hostedBy} ${providerName(s.host)}`}</span>
          <Chip tone={s.online ? "ok" : "warn"}><Dot state={s.online ? "working" : "never"} /> {s.online ? t.spaces.online : t.spaces.offline}</Chip>
        </div>
        <div className="space-actions">
          {s.mine ? <Button small kind="quiet" onClick={showInvite}>{t.spaces.invite}</Button> : <Button small kind="quiet" onClick={() => setDoor(true)}>{t.door.showAgain}</Button>}
          {confirm ? (
            <>
              <Button small kind="danger" onClick={leave}>{s.mine ? t.spaces.confirmEnd : t.spaces.confirmLeave}</Button>
              <Button small kind="quiet" onClick={() => setConfirm(false)}>{t.spaces.keep}</Button>
            </>
          ) : (
            <Button small kind="quiet" onClick={() => setConfirm(true)}>{s.mine ? t.spaces.end : t.spaces.leave}</Button>
          )}
        </div>
      </header>
      {invite ? (
        <div className="banner banner-quiet">
          <span className="muted small">{t.spaces.inviteNote}</span>
          <code className="selectable">{invite}</code>
          <Button small kind="quiet" onClick={() => navigator.clipboard?.writeText(invite).then(() => setCopied(true))}>{copied ? t.spaces.copied : t.spaces.copy}</Button>
          <button className="link" onClick={() => setInvite(null)}>{t.common.close}</button>
        </div>
      ) : null}
      {problem ? <div className="banner banner-bad">{problem}</div> : null}

      <div className="space-body">
        <section className="space-feed">
          {s.kind === "idea" && roots.some((m) => m.kind === "synthesis") ? (() => { const syn = roots.find((m) => m.kind === "synthesis")!; return (
            <div className="synthesis-pin">
              <span className="move-kind">{t.spaces.moveKinds.synthesis}</span>
              <div className="move-body"><Markdown text={syn.body} /></div>
              <span className="muted small">{syn.author === me ? t.spaces.you : syn.author} · {time(syn.at)}</span>
            </div>
          ); })() : null}
          <h2 className="list-head">{t.spaces.feed}</h2>
          {roots.length === 0 ? <p className="muted">{t.spaces.feedEmpty}</p> : null}
          {roots.map((m) => (
            <MoveCard key={m.id} move={m} replies={childrenOf(m.id)} me={me} space={space} onVerb={(name, prefill) => { const x = tool(name); if (x) begin(x, prefill); }} />
          ))}
        </section>
        <aside className="space-side">
          <section className="verbs">
            <h2 className="list-head">{t.spaces.verbs}</h2>
            <p className="muted small">{t.spaces.verbsNote}</p>
            <div className="verb-buttons">
              {space.tools.map((x) => (
                <Button key={x.name} small kind={active?.tool.name === x.name ? "primary" : "plain"} onClick={() => begin(x)} title={x.description}>{x.title}</Button>
              ))}
            </div>
            {active ? (
              <div className="verb-form">
                <div className="verb-form-head">
                  <strong>{active.tool.title}</strong>
                  <span className="muted small">{active.tool.description}</span>
                </div>
                <SchemaForm key={active.tool.name} schema={active.tool.schema as Schema} value={active.values} onChange={(v) => setActive({ ...active, values: v as Record<string, unknown> })} />
                <div className="row-actions">
                  <Button kind="primary" onClick={submit} disabled={incomplete.length > 0}>{t.spaces.do}</Button>
                  <Button kind="quiet" onClick={() => setActive(null)}>{t.spaces.cancel}</Button>
                  {incomplete.length ? <span className="muted small">{t.create.fillIn} {incomplete.join(", ")}</span> : null}
                </div>
              </div>
            ) : null}
            {said ? <p className={said.startsWith(t.spaces.did) ? "ok small" : "warn small"}>{said}</p> : null}
          </section>
          <section>
            <h2 className="list-head">{space.members.length} {t.spaces.members}</h2>
            <ul className="member-list">
              {space.members.map((m) => (
                <li key={m.name} className="member">
                  <span>{m.name === me ? t.spaces.you : m.name}</span>
                  {m.is_agent ? <Chip>{t.spaces.agent}</Chip> : null}
                  <span className="muted small">{ago(m.joined)}</span>
                </li>
              ))}
            </ul>
          </section>
          <details className="charter">
            <summary className="list-head">{t.spaces.charter}</summary>
            <Markdown text={space.charter} />
          </details>
        </aside>
      </div>
    </div>
  );
}

function fieldRows(m: MoveView) {
  const f = m.fields as Record<string, unknown>;
  return Object.entries(f).filter(([, v]) => v !== null && v !== "" && v !== undefined);
}

function MoveCard(props: { move: MoveView; replies: MoveView[]; me: string; space: SpaceView; onVerb: (name: string, prefill: Record<string, unknown>) => void }) {
  const { move: m, replies, me, space } = props;
  const isMe = m.author === me;
  const who = isMe ? t.spaces.you : m.author;
  const agent = space.members.find((x) => x.name === m.author)?.is_agent;
  const kind = t.spaces.moveKinds[m.kind] ?? m.kind;
  const state = t.spaces.states[m.state] ?? m.state;
  const has = (name: string) => space.tools.some((x) => x.name === name);
  const claimedBy = (m.fields as Record<string, unknown>).claimed_by;
  return (
    <article className={`move move-${m.kind}`}>
      <header className="move-head">
        {kind ? <span className="move-kind">{kind}</span> : null}
        <span className="move-who">{who}</span>
        {agent ? <Chip>{t.spaces.agent}</Chip> : null}
        <span className="muted small">{time(m.at)}</span>
        {state ? <Chip tone={m.state === "done" || m.state === "issued" ? "ok" : m.state === "open" ? "accent" : "plain"}>{state}</Chip> : null}
      </header>
      {m.title ? <h3 className="move-title selectable">{m.title}</h3> : null}
      {m.body ? <div className="move-body"><Markdown text={m.body} /></div> : null}
      {fieldRows(m).length ? (
        <dl className="move-fields">
          {fieldRows(m).map(([k, v]) => (
            <div key={k}><dt>{t.spaces.fields[k] ?? k}</dt><dd className="selectable">{String(v)}</dd></div>
          ))}
        </dl>
      ) : null}
      <div className="move-actions">
        {has("reply") && m.kind !== "reply" ? <Button small kind="quiet" onClick={() => props.onVerb("reply", { move_id: m.id })}>{t.spaces.reply}</Button> : null}
        {m.kind === "task" && m.state === "open" && !isMe && has("claim") ? <Button small kind="quiet" onClick={() => props.onVerb("claim", { task_id: m.id })}>{t.spaces.claim}</Button> : null}
        {m.kind === "task" && m.state === "claimed" && claimedBy === me && has("deliver") ? <Button small kind="quiet" onClick={() => props.onVerb("deliver", { task_id: m.id })}>{t.spaces.deliver}</Button> : null}
        {m.kind === "task" && m.state === "delivered" && isMe && has("accept") ? <Button small kind="quiet" onClick={() => props.onVerb("accept", { task_id: m.id })}>{t.spaces.accept}</Button> : null}
        {m.kind === "direction" && has("steer") ? (
          <>
            <Button small kind="quiet" onClick={() => props.onVerb("steer", { direction_id: m.id, move: "prefer" })}>{t.spaces.prefer}</Button>
            <Button small kind="quiet" onClick={() => props.onVerb("steer", { direction_id: m.id, move: "reject" })}>{t.spaces.reject}</Button>
            <Button small kind="quiet" onClick={() => props.onVerb("steer", { direction_id: m.id, move: "note" })}>{t.spaces.noteVerb}</Button>
          </>
        ) : null}
      </div>
      {replies.length ? (
        <div className="move-replies">
          {replies.map((r) => (
            <div key={r.id} className={`move-reply move-reply-${r.kind}`}>
              <span className="move-who">{r.author === me ? t.spaces.you : r.author}</span>
              {t.spaces.moveKinds[r.kind] ? <span className="move-kind">{t.spaces.moveKinds[r.kind]}</span> : null}
              {r.kind === "steer" ? <Chip>{t.spaces.states[r.state] ?? r.state}</Chip> : null}
              <span className="muted small">{time(r.at)}</span>
              {r.body ? <div className="move-body"><Markdown text={r.body} /></div> : null}
              {r.kind === "receipt" ? <div className="receipt-line">✓ {r.title} → {String((r.fields as Record<string, unknown>).to ?? "")}</div> : null}
            </div>
          ))}
        </div>
      ) : null}
    </article>
  );
}

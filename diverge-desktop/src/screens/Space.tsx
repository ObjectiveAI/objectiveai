import { useCallback, useEffect, useMemo, useState } from "react";
import type { AdmittedView } from "../bindings/AdmittedView";
import type { DoorwayView } from "../bindings/DoorwayView";
import type { MoveView } from "../bindings/MoveView";
import type { SpaceView } from "../bindings/SpaceView";
import type { ToolView } from "../bindings/ToolView";
import { Markdown } from "../components/Markdown";
import { SchemaForm, initial, missing, type Schema } from "../components/SchemaForm";
import { Table } from "../components/Table";
import { Button, Chip, Dot, Empty, Segmented } from "../components/ui";
import { useShared } from "../lib/context";
import { ago, providerName, time } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";

type Active = { tool: ToolView; values: Record<string, unknown> };

/** Moves the room makes about itself: shown as quiet lines, not cards. */
const SYSTEM = new Set(["admitted", "removed", "charter", "doorway"]);
/** Fields the screen shows in its own way, or not at all. */
const HIDDEN = new Set(["claimed_by_key", "statement", "thread", "poster_says", "doer_says", "key", "fingerprint", "from", "files", "agent", "take", "agree", "move", "offers", "to"]);

export function Space(props: { id: string; tabKey: string }) {
  const { refreshSpaces, spaces, open } = useShared();
  const [space, setSpace] = useState<SpaceView | null>(null);
  const [moves, setMoves] = useState<MoveView[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const [active, setActive] = useState<Active | null>(null);
  const [said, setSaid] = useState<string | null>(null);
  const [invite, setInvite] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [view, setView] = useState<"feed" | "table">("feed");
  const [fromCopy, setFromCopy] = useState(false);
  const [doorways, setDoorways] = useState<DoorwayView[]>([]);
  const [admitted, setAdmitted] = useState<AdmittedView[]>([]);
  const [vouchText, setVouchText] = useState<{ name: string; text: string } | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);
  const [moderation, setModeration] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const view = await api.space(props.id);
      setSpace(view);
      const feed = await api.spaceFeed(props.id);
      if (feed.outcome === "feed") {
        setMoves(feed.moves);
        setFromCopy(feed.from_copy);
      } else setProblem(feed.message);
      if (view.summary.online) {
        setDoorways(await api.spacesDoorways(props.id));
        if (view.summary.mine) setAdmitted(await api.spacesAdmitted(props.id));
      }
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

  const continueIt = async () => {
    const out = await api.spacesContinue(props.id);
    if (out.outcome === "hosted") {
      await refreshSpaces();
      open({ kind: "space", id: out.id });
    } else setProblem(out.message);
  };

  const vouch = async (key: string, name: string) => setVouchText({ name, text: await api.vouchFor(key, name) });

  const remove = async (key: string) => {
    const out = await api.spaceCall(props.id, "remove", { key });
    setRemoving(null);
    setModeration(out.outcome === "ok" ? t.spaces.removedNote : out.message);
    load();
  };

  const restart = async () => {
    try {
      await api.spacesRestart(props.id);
      setModeration(t.spaces.restarted);
      load();
    } catch (e) {
      setModeration(errorText(e));
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

  if (!space) return problem ? <div className="banner banner-bad">{problem}</div> : <Empty title={t.spaces.title} />;
  const s = space.summary;
  const verbs = space.tools.filter((x) => !x.host_only || s.mine);
  const incomplete = active ? missing(active.tool.schema as Schema, active.values) : [];
  const myAgents = space.members.filter((m) => m.is_agent && m.agent_of === s.you_are);
  const who = (m: { by: string; author: string }) => (m.by === s.you_key ? t.spaces.you : m.author);

  return (
    <div className="space">
      <header className="space-head">
        <div className="space-title">
          <h1>{s.title}</h1>
          <Chip>{t.spaces.kinds[s.kind] ?? s.kind}</Chip>
          <span className="muted small">{s.mine ? t.spaces.youHostIt : `${t.spaces.hostedBy} ${s.host_name} · ${providerName(s.host)}`}</span>
          <Chip tone={s.online ? "ok" : "warn"}><Dot state={s.online ? "working" : "never"} /> {s.online ? t.spaces.online : t.spaces.offline}</Chip>
        </div>
        <div className="space-actions">
          <span className="muted small">{t.spaces.youAreHere} <strong>{s.you_are}</strong>{s.fresh ? ` · ${t.spaces.freshHere}` : ""}</span>
          {s.mine ? <Button small kind="quiet" onClick={showInvite}>{t.spaces.invite}</Button> : null}
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
          <code className="selectable invite-text">{invite}</code>
          <Button small kind="quiet" onClick={() => navigator.clipboard?.writeText(invite).then(() => setCopied(true))}>{copied ? t.spaces.copied : t.spaces.copy}</Button>
          <button className="link" onClick={() => setInvite(null)}>{t.common.close}</button>
        </div>
      ) : null}
      {problem ? <div className="banner banner-bad">{problem}</div> : null}
      {!s.online ? (
        <div className="banner banner-warn">
          <span>{t.spaces.quiet} {fromCopy ? t.spaces.fromCopy : ""} </span>
          {fromCopy && !s.mine ? (
            <>
              <span className="muted small">{t.spaces.continueNote}</span>
              <Button small kind="primary" onClick={continueIt}>{t.spaces.continueIt}</Button>
            </>
          ) : null}
        </div>
      ) : null}
      {moderation ? (
        <div className="banner banner-quiet">
          <span>{moderation}</span>
          {moderation === t.spaces.removedNote ? <Button small kind="primary" onClick={restart}>{t.spaces.restartNow}</Button> : null}
          <button className="link" onClick={() => setModeration(null)}>{t.common.close}</button>
        </div>
      ) : null}
      {vouchText ? (
        <div className="banner banner-quiet">
          <strong>{t.spaces.vouchTitle} {vouchText.name}</strong>
          <span className="muted small">{t.spaces.vouchNote}</span>
          <code className="selectable invite-text">{vouchText.text}</code>
          <Button small kind="quiet" onClick={() => navigator.clipboard?.writeText(vouchText.text)}>{t.spaces.copy}</Button>
          <button className="link" onClick={() => setVouchText(null)}>{t.common.close}</button>
        </div>
      ) : null}

      <div className="space-body">
        <section className="space-feed">
          <Segmented value={view} options={[{ value: "feed", label: t.spaces.happened }, { value: "table", label: t.spaces.table }]} onChange={setView} />
          {view === "table" ? (
            <Table space={s} rooms={spaces} />
          ) : (
            <>
              {s.kind === "idea" && roots.some((m) => m.kind === "synthesis") ? (() => { const syn = roots.find((m) => m.kind === "synthesis")!; return (
                <div className="synthesis-pin">
                  <span className="move-kind">{t.spaces.moveKinds.synthesis}</span>
                  <div className="move-body"><Markdown text={syn.body} /></div>
                  <span className="muted small">{who(syn)} · {time(syn.at)}</span>
                </div>
              ); })() : null}
              {roots.length === 0 ? <p className="muted">{t.spaces.feedEmpty}</p> : null}
              {roots.map((m) =>
                SYSTEM.has(m.kind) ? (
                  <SystemLine key={m.id} move={m} you={s.you_key} />
                ) : (
                  <MoveCard key={m.id} move={m} replies={childrenOf(m.id)} space={space} onOpenInvite={(text) => open({ kind: "door", invite: text })} onVerb={(name, prefill) => { const x = tool(name); if (x) begin(x, prefill); }} />
                ),
              )}
            </>
          )}
        </section>
        <aside className="space-side">
          <section className="verbs">
            <h2 className="list-head">{t.spaces.verbs}</h2>
            <p className="muted small">{t.spaces.verbsNote}</p>
            <div className="verb-buttons">
              {verbs.map((x) => (
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
          {myAgents.length ? (
            <section>
              <h2 className="list-head">{t.spaces.yourAgents}</h2>
              <p className="muted small">{t.spaces.allowanceNote}</p>
              {myAgents.map((a) => <AllowanceRow key={a.key || a.name} room={s.id} agent={a.name} />)}
            </section>
          ) : null}
          <section>
            <h2 className="list-head">{space.members.length} {t.spaces.members}</h2>
            <ul className="member-list">
              {space.members.map((m) => (
                <li key={m.key || m.name} className="member">
                  <span>{m.key === s.you_key ? t.spaces.you : m.name}</span>
                  {m.is_agent ? <Chip>{m.agent_of === s.you_are ? t.spaces.yourAgent : `${t.spaces.runBy} ${m.agent_of ?? "?"}`}</Chip> : null}
                  <span className="muted small">{m.last_acted ? `${t.spaces.lastActed} ${ago(m.last_acted)}` : ago(m.joined)}</span>
                  {m.key && m.key !== s.you_key && !m.is_agent ? <button className="link small" onClick={() => vouch(m.key, m.name)}>{t.spaces.vouch}</button> : null}
                </li>
              ))}
            </ul>
          </section>
          {s.mine && admitted.length > 1 ? (
            <section>
              <h2 className="list-head">{t.spaces.everyone}</h2>
              <p className="muted small">{t.spaces.everyoneNote}</p>
              <ul className="member-list">
                {admitted.filter((a) => a.key !== s.you_key).map((a) => (
                  <li key={a.key} className="member">
                    <span>{a.name}</span>
                    {a.listed ? null : <Chip>{t.spaces.notListed}</Chip>}
                    {removing === a.key ? (
                      <>
                        <Button small kind="danger" onClick={() => remove(a.key)}>{t.spaces.removeConfirm}</Button>
                        <Button small kind="quiet" onClick={() => setRemoving(null)}>{t.spaces.keep}</Button>
                      </>
                    ) : (
                      <button className="link small" onClick={() => setRemoving(a.key)}>{t.spaces.remove}</button>
                    )}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          {doorways.length ? (
            <section>
              <h2 className="list-head">{t.spaces.doorways}</h2>
              <ul className="member-list">
                {doorways.map((d) => (
                  <li key={d.invite} className="member">
                    <span>{d.title}</span>
                    <button className="link small" onClick={() => open({ kind: "door", invite: d.invite })}>{t.door.openInvite}</button>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          <details className="charter">
            <summary className="list-head">{t.spaces.charter}</summary>
            <Markdown text={space.charter} />
          </details>
        </aside>
      </div>
    </div>
  );
}

function AllowanceRow(props: { room: string; agent: string }) {
  const [perDay, setPerDay] = useState<number | null>(null);
  const [used, setUsed] = useState(0);
  useEffect(() => {
    api.allowanceGet(props.room, props.agent).then((a) => {
      setPerDay(a.per_day);
      setUsed(a.used_today);
    });
  }, [props.room, props.agent]);
  if (perDay === null) return null;
  return (
    <div className="allowance">
      <span>{props.agent}</span>
      <label className="muted small">
        {t.spaces.allowance}{" "}
        <input
          type="number"
          min={0}
          max={100}
          value={perDay}
          onChange={(e) => {
            const n = Math.max(0, Math.min(100, Number(e.target.value) || 0));
            setPerDay(n);
            api.allowanceSet(props.room, props.agent, n).then((a) => setUsed(a.used_today));
          }}
        />
      </label>
      {used ? <span className="muted small">{used} {t.spaces.today}</span> : null}
    </div>
  );
}

function SystemLine(props: { move: MoveView; you: string }) {
  const m = props.move;
  const who = m.by === props.you ? t.spaces.you : m.author;
  const text =
    m.kind === "admitted" ? `${who} ${t.spaces.admitted} ${m.title}` : m.kind === "removed" ? `${m.title} ${t.spaces.removedLine}` : m.kind === "charter" ? t.spaces.rulesChanged : `${who} ${t.spaces.vouchesFor}: ${m.title}`;
  return (
    <div className="system-line muted small">
      {text} · {time(m.at)}
    </div>
  );
}

const INVITE = /diverge-invite:[A-Za-z0-9_-]+/;

function fieldRows(m: MoveView) {
  const f = m.fields as Record<string, unknown>;
  return Object.entries(f).filter(([k, v]) => !HIDDEN.has(k) && v !== null && v !== "" && v !== undefined && typeof v !== "object");
}

function MoveCard(props: { move: MoveView; replies: MoveView[]; space: SpaceView; onVerb: (name: string, prefill: Record<string, unknown>) => void; onOpenInvite: (text: string) => void }) {
  const { move: m, replies, space } = props;
  const s = space.summary;
  const isMe = m.by === s.you_key;
  const f = m.fields as Record<string, unknown>;
  const kind = t.spaces.moveKinds[m.kind] ?? m.kind;
  const state = t.spaces.states[m.state] ?? m.state;
  const has = (name: string) => space.tools.some((x) => x.name === name);
  const claimedByMe = f.claimed_by_key === s.you_key;
  const invite = m.body.match(INVITE)?.[0];
  const said = (side: unknown) => (side && typeof side === "object" ? ((side as { agree?: boolean }).agree ? t.spaces.settledWord : t.spaces.notSettledWord) : null);
  const note = (side: unknown) => (side && typeof side === "object" && typeof (side as { note?: unknown }).note === "string" ? `: “${(side as { note: string }).note}”` : "");
  const posterSays = said(f.poster_says);
  const doerSays = said(f.doer_says);
  const canSettle = m.kind === "task" && typeof f.pledge === "string" && (m.state === "delivered" || m.state === "done") && (isMe || claimedByMe) && has("settle");
  const who = (x: MoveView) => (x.by === s.you_key ? t.spaces.you : x.author);
  const agentLine = (x: MoveView) => (x.agent_of ? (x.agent_of === s.you_are ? t.spaces.yourAgent : `${t.spaces.runBy} ${x.agent_of}`) : null);
  return (
    <article className={`move move-${m.kind}`}>
      <header className="move-head">
        {kind ? <span className="move-kind">{kind}</span> : null}
        <span className="move-who">{who(m)}</span>
        {agentLine(m) ? <Chip>{agentLine(m)}</Chip> : null}
        <span className="muted small">{time(m.at)}</span>
        {state && m.kind !== "say" && m.kind !== "reply" && m.kind !== "note" ? <Chip tone={m.state === "done" || m.state === "issued" ? "ok" : m.state === "open" ? "accent" : "plain"}>{state}</Chip> : null}
        {typeof f.from === "string" ? <span className="muted small">· {f.from}</span> : null}
      </header>
      {m.title ? <h3 className="move-title selectable">{m.title}</h3> : null}
      {m.body ? <div className="move-body"><Markdown text={invite ? m.body.replace(invite, "").trim() : m.body} /></div> : null}
      {invite ? <Button small kind="plain" onClick={() => props.onOpenInvite(invite)}>{t.door.openInvite}</Button> : null}
      {fieldRows(m).length ? (
        <dl className="move-fields">
          {fieldRows(m).map(([k, v]) => (
            <div key={k}><dt>{t.spaces.fields[k] ?? k}</dt><dd className="selectable">{String(v)}</dd></div>
          ))}
        </dl>
      ) : null}
      {posterSays || doerSays ? (
        <div className="settled muted small">
          {posterSays ? <span>{who(m)} {t.spaces.says} {posterSays}{note(f.poster_says)}</span> : null}
          {doerSays ? <span>{String(f.claimed_by ?? "")} {t.spaces.says} {doerSays}{note(f.doer_says)}</span> : null}
        </div>
      ) : null}
      <div className="move-actions">
        {has("reply") && m.kind !== "reply" ? <Button small kind="quiet" onClick={() => props.onVerb("reply", { move_id: m.id })}>{t.spaces.reply}</Button> : null}
        {m.kind === "ask" && !isMe && has("offer") ? <Button small kind="quiet" onClick={() => props.onVerb("offer", { ask_id: m.id })}>{t.spaces.moveKinds.offer}</Button> : null}
        {m.kind === "task" && m.state === "open" && !isMe && has("claim") ? <Button small kind="quiet" onClick={() => props.onVerb("claim", { task_id: m.id })}>{t.spaces.claim}</Button> : null}
        {m.kind === "task" && m.state === "claimed" && claimedByMe && has("deliver") ? <Button small kind="quiet" onClick={() => props.onVerb("deliver", { task_id: m.id })}>{t.spaces.deliver}</Button> : null}
        {m.kind === "task" && m.state === "delivered" && isMe && has("accept") ? <Button small kind="quiet" onClick={() => props.onVerb("accept", { task_id: m.id })}>{t.spaces.accept}</Button> : null}
        {canSettle ? (
          <>
            <span className="muted small">{t.spaces.settle}</span>
            <Button small kind="quiet" onClick={() => props.onVerb("settle", { task_id: m.id, agree: true })}>{t.spaces.settledYes}</Button>
            <Button small kind="quiet" onClick={() => props.onVerb("settle", { task_id: m.id, agree: false })}>{t.spaces.settledNo}</Button>
          </>
        ) : null}
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
              <span className="move-who">{who(r)}</span>
              {agentLine(r) ? <Chip>{agentLine(r)}</Chip> : null}
              {t.spaces.moveKinds[r.kind] ? <span className="move-kind">{t.spaces.moveKinds[r.kind]}</span> : null}
              {r.kind === "steer" ? <Chip>{t.spaces.states[r.state] ?? r.state}</Chip> : null}
              <span className="muted small">{time(r.at)}</span>
              {r.body && r.kind !== "receipt" ? <div className="move-body"><Markdown text={r.body} /></div> : null}
              {Array.isArray((r.fields as Record<string, unknown>).files) ? <div className="muted small mono">{((r.fields as Record<string, unknown>).files as string[]).join(" · ")}</div> : null}
              {r.kind === "receipt" ? (
                <div className="receipt-line">✓ {r.title} → {String((r.fields as Record<string, unknown>).to ?? "")} <span className="muted small">· {t.spaces.receiptSealed} {s.title}</span></div>
              ) : null}
            </div>
          ))}
        </div>
      ) : null}
    </article>
  );
}

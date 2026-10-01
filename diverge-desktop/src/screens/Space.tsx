import type { AllowanceView } from "../bindings/AllowanceView";
import type { Reach } from "../bindings/Reach";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { AdmittedView } from "../bindings/AdmittedView";
import type { DoorwayView } from "../bindings/DoorwayView";
import type { MoveView } from "../bindings/MoveView";
import type { SpaceView } from "../bindings/SpaceView";
import type { ToolView } from "../bindings/ToolView";
import { Markdown } from "../components/Markdown";
import { SchemaForm, initial, missing, type Schema } from "../components/SchemaForm";
import { Table } from "../components/Table";
import { Button, Chip, Dot, Empty, SectionHead, Segmented } from "../components/ui";
import { KeyMark } from "../components/KeyMark";
import { useShared } from "../lib/context";
import { ago, providerName, time, spaceTitle } from "../lib/format";
import { api, errorText } from "../lib/ipc";
import { t } from "../strings";

type Active = { tool: ToolView; values: Record<string, unknown> };

/** Verbs that have their own place, never a raw form: letting in is at the door; removing is on a member's row;
 * rules and recommendations are under "Your room"; hires are answered on cards; a receipt is pinned from You;
 * taking back and erasing are on the move itself. */
const ELSEWHERE = new Set(["admit", "remove", "set_charter", "vouch_room", "answer_hire", "deliver_hire", "pin_receipt", "keys", "withdraw", "erase", "mark_invite", "drop_keeper"]);

/** Whether a member's key is you here: the key you seal with, or, in a room under rules 2, the account it's on. */
const isYou = (s: { you_key: string; you_account: string | null }, key?: string | null) => !!key && (key === s.you_key || key === s.you_account);

/** Moves the room makes about itself: shown as quiet lines, not cards. */
const SYSTEM = new Set(["admitted", "removed", "charter", "doorway", "keeper_dropped"]);
/** Fields the screen shows in its own way, or not at all. */
const HIDDEN = new Set(["claimed_by_key", "statement", "thread", "poster_says", "doer_says", "key", "key_mark", "fingerprint", "from", "files", "agent", "take", "agree", "move", "offers", "to", "offer", "offered_by", "taken_offer", "also", "erased", "how", "sealed", "opened"]);

export function Space(props: { id: string; tabKey: string }) {
  const { refreshSpaces, spaces, open } = useShared();
  const [space, setSpace] = useState<SpaceView | null>(null);
  const [moves, setMoves] = useState<MoveView[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const [active, setActive] = useState<Active | null>(null);
  const [said, setSaid] = useState<string | null>(null);
  const [invite, setInvite] = useState<string | null>(null);
  const [inviteIsNew, setInviteIsNew] = useState(false);
  const [copied, setCopied] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [view, setView] = useState<"feed" | "table">("feed");
  const [fromCopy, setFromCopy] = useState(false);
  const [doorways, setDoorways] = useState<DoorwayView[]>([]);
  const [admitted, setAdmitted] = useState<AdmittedView[]>([]);
  const [vouchText, setVouchText] = useState<{ name: string; room: string; text: string } | null>(null);
  const [vouching, setVouching] = useState<{ key: string; name: string; room: string } | null>(null);
  const [sentTo, setSentTo] = useState<string[]>([]);
  /** An invite to this room, said in a direct room you share with someone from before. */
  const sendInvite = async (key: string, dm: string) => {
    const inv = await api.spaceInvite(props.id);
    if (!inv || !space) return;
    const out = await api.spaceCall(dm, "say", { body: `${t.spaces.inviteMessage} ${space.summary.title}:\n\n${inv.text}` });
    if (out.outcome === "ok") setSentTo((s) => [...s, key]);
    else setProblem(out.message);
  };
  const [removing, setRemoving] = useState<string | null>(null);
  const [sayIt, setSayIt] = useState(false);
  const [editing, setEditing] = useState<string | null>(null);
  const [recommending, setRecommending] = useState<string | null>(null);
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
    } else setSaid(wordsFor(out.message));
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

  /** Rooms you could vouch someone into: the ones you're in, besides this one and direct rooms. */
  const vouchRooms = spaces.filter((r) => r.id !== props.id && r.kind !== "dm");
  const vouch = async () => {
    if (!vouching) return;
    try {
      const text = await api.vouchFor(vouching.key, vouching.name, vouching.room);
      const room = spaces.find((r) => r.id === vouching.room)?.title ?? "";
      setVouchText({ name: vouching.name, room, text });
      setVouching(null);
    } catch (e) {
      setProblem(errorText(e));
    }
  };

  const remove = async (key: string) => {
    const out = await api.spaceCall(props.id, "remove", { key, quiet: !sayIt });
    setRemoving(null);
    setSayIt(false);
    setModeration(out.outcome === "ok" ? t.spaces.removedNote : wordsFor(out.message));
    load();
  };

  const saveRules = async () => {
    if (editing === null) return;
    const out = await api.spaceCall(props.id, "set_charter", { text: editing });
    if (out.outcome === "ok") {
      setEditing(null);
      load();
    } else setProblem(wordsFor(out.message));
  };

  /** Recommend one of the rooms you host: its invite goes on this room's list of rooms it vouches for. */
  const recommend = async () => {
    if (!recommending) return;
    const inv = await api.spaceInvite(recommending);
    const room = spaces.find((r) => r.id === recommending);
    if (!inv || !room) return;
    const out = await api.spaceCall(props.id, "vouch_room", { title: spaceTitle(room), invite: inv.text });
    if (out.outcome === "ok") {
      setRecommending(null);
      setDoorways(await api.spacesDoorways(props.id));
    } else setProblem(wordsFor(out.message));
  };

  const restart = async () => {
    try {
      const fresh = await api.spacesRestart(props.id);
      setModeration(t.spaces.restarted);
      setInvite(fresh.text);
      setInviteIsNew(true);
      setCopied(false);
      load();
    } catch (e) {
      setModeration(errorText(e));
    }
  };

  const showInvite = async () => {
    const i = await api.spaceInvite(props.id);
    setInvite(i?.text ?? null);
    setInviteIsNew(false);
    setCopied(false);
  };

  const byId = useMemo(() => new Map(moves.map((m) => [m.id, m])), [moves]);
  const roots = useMemo(() => [...moves].filter((m) => !m.parent || !byId.has(m.parent)).sort((a, b) => b.at.localeCompare(a.at)), [moves, byId]);
  const childrenOf = useCallback((id: string) => moves.filter((m) => m.parent === id).sort((a, b) => a.at.localeCompare(b.at)), [moves]);
  const tool = (name: string) => space?.tools.find((x) => x.name === name);

  if (!space) return problem ? <div className="banner banner-bad">{problem}</div> : <Empty title={t.spaces.title} />;
  const s = space.summary;
  // Nothing the room would refuse you: no hiring your own agents in your own profile.
  const verbs = s.online ? space.tools.filter((x) => (!x.host_only || s.mine) && !ELSEWHERE.has(x.name) && !(x.name === "hire" && s.mine)) : [];
  // Recommending needs the room to have the verb: a direct room doesn't.
  const recommendable = tool("vouch_room") ? spaces.filter((r) => r.mine && r.id !== props.id && r.kind !== "dm") : [];
  const lastSeen = moves.reduce<string | null>((a, m) => (!a || m.at > a ? m.at : a), null);
  const incomplete = active ? missing(active.tool.schema as Schema, active.values) : [];
  // Your agents here: the app gives each its slot, which their allowances go by.
  const myAgents = space.members.filter((m) => m.is_agent && m.slot);
  const who = (m: { by: string; author: string }) => (m.by === s.you_key ? t.spaces.you : m.author);

  return (
    <div className="space">
      <header className="space-head">
        <div className="space-title">
          <h1>{spaceTitle(s)}</h1>
          <Chip>{t.spaces.kinds[s.kind] ?? s.kind}</Chip>
          <span className="muted small">{s.mine ? t.spaces.youHostIt : `${t.spaces.hostedBy} ${s.host_name} · ${providerName(s.host)}`}</span>
          <Chip tone={s.online ? "ok" : "warn"}><Dot state={s.online ? "working" : "never"} /> {s.online ? t.spaces.online : t.spaces.offline}</Chip>
        </div>
        <div className="space-actions">
          <span className="muted small">{t.spaces.youAreHere} <strong>{s.you_are}</strong>{s.fresh ? ` · ${t.spaces.freshHere}` : ""}</span>
          {s.mine ? <Button small kind="tertiary" onClick={showInvite}>{t.spaces.invite}</Button> : null}
          {confirm ? (
            <>
              <Button small kind="danger" onClick={leave}>{s.mine ? t.spaces.confirmEnd : t.spaces.confirmLeave}</Button>
              <Button small kind="tertiary" onClick={() => setConfirm(false)}>{t.spaces.keep}</Button>
            </>
          ) : (
            <Button small kind="tertiary" onClick={() => setConfirm(true)}>{s.mine ? t.spaces.end : t.spaces.leave}</Button>
          )}
        </div>
      </header>
      {invite ? (
        <div className="banner banner-quiet">
          <span className="muted small">{inviteIsNew ? t.spaces.newInviteNote : t.spaces.inviteNote}</span>
          <code className="selectable invite-text">{invite}</code>
          <Button small kind="tertiary" onClick={() => navigator.clipboard?.writeText(invite).then(() => setCopied(true))}>{copied ? t.spaces.copied : t.spaces.copy}</Button>
          <Button small kind="tertiary" onClick={() => setInvite(null)}>{t.common.close}</Button>
        </div>
      ) : null}
      {problem ? <div className="banner banner-bad">{problem}</div> : null}
      {!s.online ? (
        <div className="banner banner-warn">
          <span>
            {[t.spaces.quiet, fromCopy && lastSeen ? `${t.spaces.copyAsOf} ${time(lastSeen)}.` : "", fromCopy ? t.spaces.fromCopy : ""].filter(Boolean).join(" ")}
          </span>
          {fromCopy && !s.mine ? (
            <>
              <span className="muted small"> {t.spaces.continueNote}</span>
              <Button small kind="primary" onClick={continueIt}>{t.spaces.continueIt}</Button>
            </>
          ) : null}
        </div>
      ) : null}
      {moderation ? (
        <div className="banner banner-quiet">
          <span>{moderation}</span>
          {moderation === t.spaces.removedNote ? <Button small kind="primary" onClick={restart}>{t.spaces.restartNow}</Button> : null}
          <Button small kind="tertiary" onClick={() => setModeration(null)}>{t.common.close}</Button>
        </div>
      ) : null}
      {vouchText ? (
        <div className="banner banner-quiet">
          <strong>{t.spaces.vouchTitle} {vouchText.name} → {vouchText.room}</strong>
          <span className="muted small">{t.spaces.vouchNote}</span>
          <code className="selectable invite-text">{vouchText.text}</code>
          <Button small kind="tertiary" onClick={() => navigator.clipboard?.writeText(vouchText.text)}>{t.spaces.copy}</Button>
          <Button small kind="tertiary" onClick={() => setVouchText(null)}>{t.common.close}</Button>
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
          {verbs.length ? (
          <section className="verbs">
            <SectionHead small title={t.spaces.verbs} />
            <p className="muted small">{t.spaces.verbsNote}</p>
            <div className="verb-buttons">
              {verbs.map((x) => (
                <Button key={x.name} small kind={active?.tool.name === x.name ? "primary" : "secondary"} onClick={() => begin(x)} title={x.description}>{t.spaces.verbNames[x.name] ?? x.title}</Button>
              ))}
            </div>
            {active ? (
              <div className="verb-form">
                <div className="verb-form-head">
                  <strong>{t.spaces.verbNames[active.tool.name] ?? active.tool.title}</strong>
                  <span className="muted small">{active.tool.description}</span>
                </div>
                <SchemaForm key={active.tool.name} schema={active.tool.schema as Schema} value={active.values} onChange={(v) => setActive({ ...active, values: v as Record<string, unknown> })} />
                <div className="row-actions">
                  <Button kind="primary" onClick={submit} disabled={incomplete.length > 0}>{t.spaces.do}</Button>
                  <Button kind="tertiary" onClick={() => setActive(null)}>{t.spaces.cancel}</Button>
                  {incomplete.length ? <span className="muted small">{t.create.fillIn} {incomplete.join(", ")}</span> : null}
                </div>
              </div>
            ) : null}
            {said ? <p className={said.startsWith(t.spaces.did) ? "ok small" : "warn small"}>{said}</p> : null}
          </section>
          ) : null}
          {s.mine && s.online ? (
            <section>
              <SectionHead small title={t.spaces.yourRoom} />
              <div className="row-actions">
                <Button small kind="secondary" onClick={() => setEditing(editing === null ? space.charter : null)}>{t.spaces.editRules}</Button>
                {recommendable.length ? <Button small kind="secondary" onClick={() => setRecommending(recommending ? null : recommendable[0].id)}>{t.spaces.recommendRoom}</Button> : null}
              </div>
              {editing !== null ? (
                <div className="verb-form">
                  <p className="muted small">{t.spaces.rulesNote}</p>
                  <textarea className="rules-edit" rows={10} value={editing} onChange={(e) => setEditing(e.target.value)} />
                  <RulesChange before={space.charter} after={editing} />
                  <div className="row-actions">
                    <Button kind="primary" onClick={saveRules} disabled={editing.trim() === space.charter.trim() || !editing.trim()}>{t.spaces.saveRules}</Button>
                    <Button kind="tertiary" onClick={() => setEditing(null)}>{t.common.cancel}</Button>
                  </div>
                </div>
              ) : null}
              {recommending ? (
                <div className="verb-form">
                  <p className="muted small">{t.spaces.recommendNote}</p>
                  <select value={recommending} onChange={(e) => setRecommending(e.target.value)}>
                    {recommendable.map((r) => <option key={r.id} value={r.id}>{spaceTitle(r)}</option>)}
                  </select>
                  <div className="row-actions">
                    <Button kind="primary" onClick={recommend}>{t.spaces.recommendIt}</Button>
                    <Button kind="tertiary" onClick={() => setRecommending(null)}>{t.common.cancel}</Button>
                  </div>
                </div>
              ) : null}
            </section>
          ) : null}
          {myAgents.length ? (
            <section>
              <SectionHead small title={t.spaces.yourAgents} />
              <p className="muted small">{t.spaces.allowanceNote}</p>
              {myAgents.map((a) => <AllowanceRow key={a.key} room={s.id} slot={a.slot!} name={a.name} />)}
            </section>
          ) : null}
          <section>
            <SectionHead small title={`${space.members.length} ${s.online ? t.spaces.members : t.spaces.membersWhenSeen}`} />
            <ul className="member-list">
              {space.members.map((m) => (
                <li key={m.key || m.name} className="member">
                  <span>{isYou(s, m.key) ? t.spaces.you : m.name}</span>
                  <KeyMark mark={m.mark} />
                  {m.is_agent ? <Chip>{isYou(s, m.agent_of_key) ? t.spaces.yourAgent : `${t.spaces.runBy} ${m.agent_of ?? "?"}`}</Chip> : null}
                  <span className="muted small">{m.last_acted ? `${t.spaces.lastActed} ${ago(m.last_acted)}` : ago(m.joined)}</span>
                  {m.key && !isYou(s, m.key) && !m.is_agent && vouchRooms.length > 0 ? (
                    <Button small kind="tertiary" onClick={() => setVouching({ key: m.key, name: m.name, room: vouchRooms[0].id })}>{t.spaces.vouch}</Button>
                  ) : null}
                  {vouching?.key === m.key ? (
                    <div className="vouch-pick">
                      <label className="muted small">
                        {t.spaces.vouchInto}{" "}
                        <select value={vouching.room} onChange={(e) => setVouching({ ...vouching, room: e.target.value })}>
                          {vouchRooms.map((r) => <option key={r.id} value={r.id}>{spaceTitle(r)}</option>)}
                        </select>
                      </label>
                      <Button small kind="primary" onClick={vouch}>{t.spaces.vouchMake}</Button>
                      <Button small kind="tertiary" onClick={() => setVouching(null)}>{t.common.cancel}</Button>
                    </div>
                  ) : null}
                </li>
              ))}
            </ul>
          </section>
          {space.before.length > 0 ? (
            <section>
              <SectionHead small title={t.spaces.fromBefore} />
              <p className="muted small">{t.spaces.fromBeforeNote}</p>
              <ul className="member-list">
                {space.before.map((b) => (
                  <li key={b.key} className="member">
                    <span>{b.name}</span>
                    {sentTo.includes(b.key) ? <Chip tone="ok">{t.spaces.inviteSent}</Chip> : b.dm ? <Button small kind="tertiary" onClick={() => sendInvite(b.key, b.dm!)}>{t.spaces.sendInvite}</Button> : <span className="muted small">{t.spaces.noDm}</span>}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          {s.mine && admitted.some((a) => !isYou(s, a.key) && !a.yours) ? (
            <section>
              <SectionHead small title={t.spaces.everyone} />
              <p className="muted small">{t.spaces.everyoneNote}</p>
              <ul className="member-list">
                {admitted.filter((a) => !isYou(s, a.key) && !a.yours).map((a) => (
                  <li key={a.key} className="member">
                    <span>{a.name}</span>
                    <KeyMark mark={a.mark} />
                    {a.listed ? null : <Chip>{t.spaces.notListed}</Chip>}
                    {removing === a.key ? (
                      <div className="remove-confirm">
                        <label className="switch small">
                          <input type="checkbox" checked={sayIt} onChange={(e) => setSayIt(e.target.checked)} />
                          <span>{t.spaces.sayItInRoom}</span>
                        </label>
                        <Button small kind="danger" onClick={() => remove(a.key)}>{t.spaces.removeConfirm} {a.name}</Button>
                        <Button small kind="tertiary" onClick={() => setRemoving(null)}>{t.spaces.keep}</Button>
                      </div>
                    ) : (
                      <Button small kind="tertiary" onClick={() => setRemoving(a.key)}>{t.spaces.remove} {a.name}</Button>
                    )}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          {doorways.length ? (
            <section>
              <SectionHead small title={t.spaces.doorways} />
              <ul className="member-list">
                {doorways.map((d) => (
                  <li key={d.invite} className="member">
                    <span>{d.title}</span>
                    <Button small kind="tertiary" onClick={() => open({ kind: "door", invite: d.invite })}>{t.door.openInvite}</Button>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          <details className="charter">
            <summary className="charter-head">{t.spaces.charter}</summary>
            <p className="muted small">{s.mine ? t.spaces.rulesYours : `${t.spaces.rulesSetBy} ${s.host_name}. ${t.spaces.rulesKeep}`}</p>
            <Markdown text={space.charter} />
          </details>
        </aside>
      </div>
    </div>
  );
}

const REACHES: Reach[] = ["read", "talk", "work", "pledge"];

/** One agent's allowance in a room, kept by its slot: never by the name it goes by there. */
function AllowanceRow(props: { room: string; slot: string; name: string }) {
  const [a, setA] = useState<AllowanceView | null>(null);
  useEffect(() => {
    api.allowanceGet(props.room, props.slot).then(setA);
  }, [props.room, props.slot]);
  if (!a) return null;
  return (
    <div className="allowance">
      <span>{props.name}</span>
      <span className="muted small">{t.spaces.allowance}</span>
      {REACHES.map((r) => (
        <label key={r} className="muted small allowance-kind">
          <input
            type="number"
            min={0}
            max={100}
            value={a.per_day[r] ?? 0}
            onChange={(e) => {
              const n = Math.max(0, Math.min(100, Number(e.target.value) || 0));
              setA({ ...a, per_day: { ...a.per_day, [r]: n } });
              api.allowanceSet(props.room, props.slot, r, n).then(setA);
            }}
          />{" "}
          {t.spaces.reaches[r]}
          {a.used_today[r] ? ` (${a.used_today[r]} ${t.spaces.today})` : ""}
        </label>
      ))}
    </div>
  );
}

function SystemLine(props: { move: MoveView; you: string }) {
  const m = props.move;
  const who = m.by === props.you ? t.spaces.you : m.author;
  const text =
    m.kind === "admitted" ? ((m.fields as Record<string, unknown>).key === props.you ? `${who} ${t.spaces.letYouIn}` : `${who} ${t.spaces.admitted} ${m.title}`) : m.kind === "removed" ? `${m.title} ${t.spaces.removedLine}` : m.kind === "charter" ? t.spaces.rulesChanged : m.kind === "keeper_dropped" ? `${who} ${t.spaces.keeperDropped}` : `${who} ${t.spaces.vouchesFor}: ${m.title}`;
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
  const isMe = isYou(s, m.member ?? m.by);
  const f = m.fields as Record<string, unknown>;
  const kind = t.spaces.moveKinds[m.kind] ?? m.kind;
  const state = t.spaces.states[m.state] ?? m.state;
  const has = (name: string) => space.tools.some((x) => x.name === name);
  const claimedByMe = isYou(s, f.claimed_by_key as string | undefined);
  const invite = m.body.match(INVITE)?.[0];
  const said = (side: unknown) => (side && typeof side === "object" ? ((side as { agree?: boolean }).agree ? t.spaces.settledWord : t.spaces.notSettledWord) : null);
  const note = (side: unknown) => (side && typeof side === "object" && typeof (side as { note?: unknown }).note === "string" ? `: “${(side as { note: string }).note}”` : "");
  const posterSays = said(f.poster_says);
  const doerSays = said(f.doer_says);
  const canSettle = m.kind === "task" && typeof f.pledge === "string" && (m.state === "delivered" || m.state === "done") && (isMe || claimedByMe) && has("settle");
  /** Under rules 2 a move names its maker's account, so it's yours from any of your devices. */
  const mine = (x: MoveView) => isYou(s, x.member ?? x.by);
  const who = (x: MoveView) => (mine(x) ? t.spaces.you : x.author);
  const agentLine = (x: MoveView) => (x.agent_of ? (x.agent_of === s.you_are ? t.spaces.yourAgent : `${t.spaces.runBy} ${x.agent_of}`) : null);
  /** Taking back your own words, or, for the host, erasing someone else's with a reason. */
  const erasing = (x: MoveView) => {
    // The room says which moves it would erase now.
    if (!x.erasable) return null;
    if (mine(x) && has("withdraw")) return <Button small kind="tertiary" onClick={() => props.onVerb("withdraw", { move_id: x.id })}>{t.spaces.takeBack}</Button>;
    if (!mine(x) && s.mine && has("erase")) return <Button small kind="tertiary" onClick={() => props.onVerb("erase", { move_id: x.id })}>{t.spaces.erase}</Button>;
    return null;
  };
  const erasedNote = (x: MoveView) => ((x.fields as Record<string, unknown>).erased ? <p className="muted small">{t.spaces.wordsErased} {t.spaces.erasedCopies}</p> : null);
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
      {erasedNote(m)}
      {f.sealed === true ? <p className="muted small">{f.opened === true ? t.spaces.sealedOpen : t.spaces.sealedShut}</p> : null}
      {m.body ? <div className="move-body"><Markdown text={invite ? m.body.replace(invite, "").trim() : m.body} /></div> : null}
      {invite ? <Button small kind="secondary" onClick={() => props.onOpenInvite(invite)}>{t.door.openInvite}</Button> : null}
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
        {has("reply") && m.kind !== "reply" && f.sealed !== true ? <Button small kind="tertiary" onClick={() => props.onVerb("reply", { move_id: m.id })}>{t.spaces.reply}</Button> : null}
        {m.kind === "ask" && !isMe && has("offer") ? <Button small kind="tertiary" onClick={() => props.onVerb("offer", { ask_id: m.id })}>{t.spaces.moveKinds.offer}</Button> : null}
        {m.kind === "task" && m.state === "open" && !isMe && has("claim") ? <Button small kind="tertiary" onClick={() => props.onVerb("claim", { task_id: m.id })}>{t.spaces.claim}</Button> : null}
        {m.kind === "task" && m.state === "claimed" && claimedByMe && has("deliver") ? <Button small kind="tertiary" onClick={() => props.onVerb("deliver", { task_id: m.id })}>{t.spaces.deliver}</Button> : null}
        {m.kind === "task" && m.state === "delivered" && isMe && has("accept") ? <Button small kind="tertiary" onClick={() => props.onVerb("accept", { task_id: m.id })}>{t.spaces.accept}</Button> : null}
        {canSettle ? (
          <>
            <span className="muted small">{t.spaces.settle}</span>
            <Button small kind="tertiary" onClick={() => props.onVerb("settle", { task_id: m.id, agree: true })}>{t.spaces.settledYes}</Button>
            <Button small kind="tertiary" onClick={() => props.onVerb("settle", { task_id: m.id, agree: false })}>{t.spaces.settledNo}</Button>
          </>
        ) : null}
        {m.kind === "direction" && has("steer") ? (
          <>
            <Button small kind="tertiary" onClick={() => props.onVerb("steer", { direction_id: m.id, move: "prefer" })}>{t.spaces.prefer}</Button>
            <Button small kind="tertiary" onClick={() => props.onVerb("steer", { direction_id: m.id, move: "reject" })}>{t.spaces.reject}</Button>
            <Button small kind="tertiary" onClick={() => props.onVerb("steer", { direction_id: m.id, move: "note" })}>{t.spaces.noteVerb}</Button>
          </>
        ) : null}
        {erasing(m)}
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
              {r.kind === "erased" ? <p className="muted small">{t.spaces.erasedCopies}</p> : null}
              {erasedNote(r)}
              {erasing(r)}
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

/** What a change to the rules does: lines added and taken away, before anyone else sees it. */
function RulesChange(props: { before: string; after: string }) {
  const was = props.before.split("\n");
  const now = props.after.split("\n");
  const gone = was.filter((l) => l.trim() && !now.includes(l));
  const added = now.filter((l) => l.trim() && !was.includes(l));
  if (!gone.length && !added.length) return <p className="muted small">{t.spaces.rulesSame}</p>;
  return (
    <div className="rules-change small">
      {gone.map((l, i) => <div key={`g${i}`} className="rules-gone">− {l}</div>)}
      {added.map((l, i) => <div key={`a${i}`} className="rules-added">+ {l}</div>)}
    </div>
  );
}

/** A room's refusal, in the screen's words where we know it. */
function wordsFor(message: string): string {
  for (const [pattern, words] of t.spaces.refusals) if (message.includes(pattern)) return words;
  return message;
}

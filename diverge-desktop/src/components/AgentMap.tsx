import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode, type RefObject } from "react";
import type { AgentView } from "../bindings/AgentView";
import type { LogEntry } from "../bindings/LogEntry";
import type { MapNode } from "../bindings/MapNode";
import { agentLabel, ago, kindTitle, providerName } from "../lib/format";
import { api } from "../lib/ipc";
import { opens } from "../lib/keys";
import { doing, places, step, tailFrom, type Layout } from "../lib/map";
import { tails } from "../lib/tails";
import { t } from "../strings";
import { Chip, Dot, Row } from "./ui";

/** The map of who made whom: an org chart drawn top-down or across, panned by dragging with the mouse
 *  (the window's own zoom keys make it bigger or smaller). A tree to the keyboard: Tab reaches one card and
 *  every control on every card; arrow keys move between cards; Enter opens a card's conversation. */
export function AgentMap(props: { tree: MapNode[]; layout: Layout; onOpen: (agent: AgentView) => void; actions?: (agent: AgentView) => ReactNode }) {
  const canvas = useRef<HTMLDivElement>(null);
  const [rover, setRover] = useState<string | null>(null);
  const { order } = useMemo(() => places(props.tree), [props.tree]);
  const stop = rover !== null && order.includes(rover) ? rover : (order[0] ?? null);
  const pan = usePan(canvas);

  const onKey = (e: KeyboardEvent<HTMLDivElement>, node: MapNode) => {
    if (e.target !== e.currentTarget) return; // keys on a card's own controls are theirs
    if (opens(e.key)) {
      if (node.what.kind === "agent") {
        e.preventDefault();
        props.onOpen(node.what.agent);
      }
      return;
    }
    const to = step(props.tree, node.key, e.key, props.layout);
    if (to === null) return;
    e.preventDefault();
    setRover(to);
    const el = canvas.current?.querySelector<HTMLElement>(`[data-node="${CSS.escape(to)}"]`);
    el?.focus({ preventScroll: true });
    el?.scrollIntoView({ block: "nearest", inline: "nearest" });
  };

  const node = (n: MapNode, level: number, at: number, of: number): ReactNode => (
    <div key={n.key} className="map-node">
      <div className="map-self">
        <div
          role="treeitem"
          data-node={n.key}
          tabIndex={stop === n.key ? 0 : -1}
          aria-level={level}
          aria-posinset={at + 1}
          aria-setsize={of}
          aria-expanded={n.children.length ? true : undefined}
          aria-owns={n.children.length ? `map-under-${n.key}` : undefined}
          aria-label={nodeLabel(n)}
          className={`map-card${n.what.kind === "agent" ? "" : " map-card-group"}`}
          onKeyDown={(e) => onKey(e, n)}
          onFocus={() => setRover(n.key)}
        >
          {n.what.kind === "agent" ? (
            <AgentCard agent={n.what.agent} cards={n.what.cards} canvas={canvas} onOpen={() => props.onOpen((n.what as { agent: AgentView }).agent)} panned={pan.moved} actions={props.actions} />
          ) : (
            <GroupCard node={n} />
          )}
        </div>
      </div>
      {n.children.length ? (
        <>
          <div className="map-stem" aria-hidden />
          <div className="map-children" role="group" id={`map-under-${n.key}`}>
            {n.children.map((c, i) => (
              <div key={c.key} className="map-child">
                {n.children.length > 1 ? (
                  <div className="map-rail" aria-hidden>
                    <div className={`map-rail-lead${i === 0 ? " map-rail-off" : ""}`} />
                    <div className={`map-rail-trail${i === n.children.length - 1 ? " map-rail-off" : ""}`} />
                  </div>
                ) : null}
                <div className="map-child-body">
                  <div className="map-stem" aria-hidden />
                  {node(c, level + 1, i, n.children.length)}
                </div>
              </div>
            ))}
          </div>
        </>
      ) : null}
    </div>
  );

  return (
    <div
      ref={canvas}
      className={`map-canvas map-${props.layout}${pan.panning ? " panning" : ""}`}
      onPointerDown={pan.down}
      onPointerMove={pan.move}
      onPointerUp={pan.end}
      onPointerCancel={pan.end}
    >
      <div className="map-forest" role="tree" aria-label={t.map.canvas}>
        {props.tree.map((n, i) => node(n, 1, i, props.tree.length))}
      </div>
    </div>
  );
}

/** What a node is called, for a screen reader moving through the tree. */
function nodeLabel(n: MapNode): string {
  switch (n.what.kind) {
    case "agent": {
      const a = n.what.agent;
      const state = a.active ? t.status.working : a.last_active ? t.status.idle : t.status.never;
      return `${agentLabel(a)}, ${state}${n.what.cards ? `, ${n.what.cards} ${t.map.waitingOnYou}` : ""}`;
    }
    case "tool":
      return toolName(n);
    case "elsewhere":
      return t.map.elsewhere;
  }
}

function toolName(n: MapNode): string {
  if (n.what.kind !== "tool") return "";
  const m = n.what.maker;
  switch (m.kind) {
    case "tool":
      return m.name ?? `${t.map.aTool} #${m.index}`;
    case "dependency_tool":
      return t.map.dependencyTool;
    case "client":
    case "agent":
      return t.map.aTool;
  }
}

/** A tool of yours that made agents, or the group of agents whose maker isn't in your list. */
function GroupCard({ node }: { node: MapNode }) {
  return (
    <div className="map-group">
      <span className="map-name">{node.what.kind === "tool" ? toolName(node) : t.map.elsewhere}</span>
      <span className="muted small">{node.what.kind === "tool" ? t.map.toolNote : t.map.elsewhereNote}</span>
    </div>
  );
}

/** One agent: its state as the rail shows it, its machine, when it last ran, what it's doing (or last said),
 *  its helpers while it works, its tools and tags, and what waits on you. */
function AgentCard(props: { agent: AgentView; cards: number; canvas: RefObject<HTMLDivElement | null>; onOpen: () => void; panned: () => boolean; actions?: (agent: AgentView) => ReactNode }) {
  const a = props.agent;
  const self = useRef<HTMLDivElement>(null);
  const seen = useOnScreen(self, props.canvas);
  const entries = useTail(a, seen);
  const now = useMemo(() => (entries ? doing(entries, a.active) : null), [entries, a.active]);
  const state = a.active ? "working" : a.last_active ? "idle" : "never";
  const working = a.active;
  return (
    // A click anywhere on the card opens it too, but not one that ends a drag, lands on a control, or selects words.
    <div ref={self} className="map-agent" onClick={(e) => { if (!props.panned() && !(e.target as Element).closest("button, a, input, select, textarea, .selectable")) props.onOpen(); }}>
      <Row className="map-card-head" onClick={props.onOpen} title={`${t.map.open} ${agentLabel(a)}`}>
        <Dot state={state} />
        <span className="map-name">{agentLabel(a)}</span>
        <span className="map-kind">{kindTitle(a.image_name)}</span>
      </Row>
      <p className="map-meta muted">
        {a.provider ? `${working ? t.status.on : t.map.lastOn} ${providerName(a.provider)}` : t.status.never}
        {a.last_active ? ` · ${ago(a.last_active)}` : ""}
      </p>
      {now?.line ? (
        <p className={`map-line selectable${working ? " live" : ""}`}>
          {working ? <span className="working-dots small"><i /><i /><i /></span> : null} {now.line}
        </p>
      ) : entries === null && a.last_active ? (
        <p className="map-line muted">{t.map.reading}</p>
      ) : a.last_active ? (
        <p className="map-line muted">{t.map.nothingSaid}</p>
      ) : null}
      {!working && now && now.helpers.length ? (
        <p className="map-meta muted">{now.helpers.length} {now.helpers.length > 1 ? t.map.helpers : t.map.helperOne}</p>
      ) : null}
      {a.tools.length || a.tags.length || props.cards ? (
        <div className="map-chips">
          {props.cards ? <Chip tone="accent">{props.cards} {t.map.waitingOnYou}</Chip> : null}
          {a.tools.map((x) => <Chip key={`tool-${x}`} title={t.map.tools}>{x}</Chip>)}
          {a.tags.map((x) => <Chip key={`tag-${x}`} title={t.map.tags}>{t.map.tagMark}{x}</Chip>)}
        </div>
      ) : null}
      {props.actions?.(a)}
      {working && now && now.helpers.length ? (
        <ul className="map-helpers" aria-label={`${now.helpers.length} ${now.helpers.length > 1 ? t.map.helpers : t.map.helperOne}`}>
          {now.helpers.map((h) => (
            <li key={h.id} className="map-helper">
              <span className="map-helper-name mono">{h.name}</span>
              {h.done ? <span className="muted"> · {t.map.helperDone}</span> : null}
              {h.line ? <span className="map-helper-line">{h.line}</span> : null}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

/** Whether an element is in the canvas's view: idle cards read their log only when someone could see them. */
function useOnScreen(el: RefObject<HTMLElement | null>, root: RefObject<HTMLElement | null>): boolean {
  const [on, setOn] = useState(false);
  useEffect(() => {
    const node = el.current;
    if (!node || typeof IntersectionObserver === "undefined") return;
    const seen = new IntersectionObserver((events) => setOn(events.some((e) => e.isIntersecting)), { root: root.current, rootMargin: "120px" });
    seen.observe(node);
    return () => seen.disconnect();
  }, [el, root]);
  return on;
}

/** The tail of an agent's log: watched while it works (until its run ends or the card goes), read once
 *  while it's idle and on screen. No more than a dozen at once; past that, a card waits its turn. */
function useTail(agent: AgentView, onScreen: boolean): LogEntry[] | null {
  const [entries, setEntries] = useState<LogEntry[] | null>(null);
  const watch = agent.active;
  const from = tailFrom(agent.logs_index);
  const wanted = watch || onScreen;
  useEffect(() => {
    if (!wanted) return;
    let scope: string | null = null;
    let closed = false;
    const turn = tails.take();
    const got: LogEntry[] = [];
    turn.ready.then(() => {
      if (closed) return;
      api
        .logsOpen({ name: agent.key, logs_index_from: from, logs_index_to: null, created_from: null, created_to: null, item_type: null, count: null, watch }, (event) => {
          if (event.event === "entry") {
            got.push(event.entry);
            if (watch) setEntries([...got]);
          } else if (event.event === "end") {
            setEntries([...got]);
            turn.done();
          } else setEntries([...got]);
        })
        .then((id) => {
          scope = id;
          if (closed) api.scopeClose(id);
        })
        .catch(() => turn.done());
    });
    return () => {
      closed = true;
      turn.done();
      if (scope) api.scopeClose(scope);
    };
  }, [agent.key, from, watch, wanted]);
  return entries;
}

/** Mouse-only drag to pan the canvas: a press on a card's controls or on words you can select is left alone,
 *  and nothing is captured until the mouse has really moved, so a click stays a click. */
function usePan(canvas: RefObject<HTMLDivElement | null>) {
  const from = useRef<{ x: number; y: number; left: number; top: number } | null>(null);
  const [panning, setPanning] = useState(false);
  // Whether the last press became a drag: the click that ends it opens nothing.
  const dragged = useRef(false);
  const down = (e: PointerEvent<HTMLDivElement>) => {
    if (e.pointerType !== "mouse" || e.button !== 0) return;
    if ((e.target as Element).closest("button, a, input, select, textarea, .selectable")) return;
    const el = canvas.current;
    dragged.current = false;
    if (el) from.current = { x: e.clientX, y: e.clientY, left: el.scrollLeft, top: el.scrollTop };
  };
  const move = (e: PointerEvent<HTMLDivElement>) => {
    const start = from.current;
    const el = canvas.current;
    if (!start || !el) return;
    const dx = e.clientX - start.x;
    const dy = e.clientY - start.y;
    if (!panning && Math.abs(dx) + Math.abs(dy) > 3) {
      setPanning(true);
      dragged.current = true;
      try {
        el.setPointerCapture(e.pointerId);
      } catch {
        // Capture is best-effort.
      }
    }
    el.scrollLeft = start.left - dx;
    el.scrollTop = start.top - dy;
  };
  const end = (e: PointerEvent<HTMLDivElement>) => {
    from.current = null;
    setPanning(false);
    try {
      canvas.current?.releasePointerCapture(e.pointerId);
    } catch {
      // Never captured.
    }
  };
  return { panning, down, move, end, moved: () => dragged.current };
}

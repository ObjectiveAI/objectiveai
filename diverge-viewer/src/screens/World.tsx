import { daemon } from "@diverge/sdk";
import { Empty } from "../components/ui";
import { useShared } from "../lib/context";
import { type ListSpec, useListing } from "../lib/daemon";
import { t } from "../strings";
import { NotConnected } from "./NotConnected";

// A first picture of the daemon, to be replaced: the agents and the
// tools are the nodes, the events between them the edges, drawn as one
// SVG with the nodes on a circle. No layout library, nothing shared.

const tools = daemon.endpoints.standalone_tools.list;
const events = daemon.endpoints.events.list;
type Tool = daemon.endpoints.standalone_tools.list.server.response.Tool;
type Event = daemon.endpoints.events.Event;
type Creator = daemon.creator.Creator;

const TOOLS: ListSpec<Tool, daemon.endpoints.standalone_tools.list.server.response.Frame> = {
  open: (h) => tools.client.execute(h, {}),
  key: tools.server.response.key,
  change: tools.server.response.change,
};

const EVENTS: ListSpec<Event, daemon.endpoints.events.list.server.response.Frame> = {
  open: (h) => events.client.execute(h, {}),
  key: daemon.endpoints.events.key,
  change: events.server.response.change,
};

type Kind = "client" | "agent" | "tool";
type Node = { key: string; kind: Kind; label: string };

/** The node a party of an event is: its key, its kind and its label. */
function nodeOf(party: Creator): Node {
  switch (party.type) {
    case "client":
      return { key: `client:${party.identity.issuer}:${party.identity.subject}`, kind: "client", label: party.identity.subject };
    case "agent":
      return { key: `agent:${party.template}:${party.index}`, kind: "agent", label: party.name ?? `${party.template} #${party.index}` };
    case "tool":
      if ("record" in party) {
        const r = party.record;
        return { key: `tool:${r.template}:${r.index}`, kind: "tool", label: r.name ?? `${r.template} #${r.index}` };
      }
      return { key: `dep:${party.dependency.template}:${JSON.stringify(party.dependency.owner)}`, kind: "tool", label: party.dependency.template };
  }
}

/** The node a listed tool is. */
function toolNode(tool: Tool): Node {
  const o = tool.origin;
  if (o.kind === "created") return { key: `tool:${o.template}:${o.index}`, kind: "tool", label: tool.name ?? `${o.template} #${o.index}` };
  return { key: `connected:${o.daemon}:${o.index}`, kind: "tool", label: tool.name ?? `${o.daemon} #${o.index}` };
}

const ORDER: Record<Kind, number> = { client: 0, agent: 1, tool: 2 };
const SIZE = 640;
const CENTER = SIZE / 2;
const RADIUS = 220;
const NODE = 9;

export function World() {
  const { connected, agents } = useShared();
  const toolsHeld = useListing(TOOLS, connected);
  const eventsHeld = useListing(EVENTS, connected);
  if (!connected) return <NotConnected title={t.world.title} />;

  const nodes = new Map<string, Node>();
  for (const a of agents.items) nodes.set(`agent:${a.template}:${a.index}`, { key: `agent:${a.template}:${a.index}`, kind: "agent", label: a.name ?? `${a.template} #${a.index}` });
  for (const tool of toolsHeld.items) {
    const n = toolNode(tool);
    nodes.set(n.key, n);
  }
  const edges = eventsHeld.items.map((e) => {
    const from = nodeOf(e.sender);
    const to = nodeOf(e.receiver);
    if (!nodes.has(from.key)) nodes.set(from.key, from);
    if (!nodes.has(to.key)) nodes.set(to.key, to);
    return { id: e.id, from: from.key, to: to.key, label: `${t.world.actions[e.action]} ×${e.count}` };
  });

  // Stable order — by kind, then key — so a node keeps its place when another arrives.
  const ordered = [...nodes.values()].sort((a, b) => ORDER[a.kind] - ORDER[b.kind] || a.key.localeCompare(b.key));
  const at = new Map<string, { x: number; y: number }>();
  ordered.forEach((n, i) => {
    const angle = (2 * Math.PI * i) / ordered.length - Math.PI / 2;
    at.set(n.key, ordered.length === 1 ? { x: CENTER, y: CENTER } : { x: CENTER + RADIUS * Math.cos(angle), y: CENTER + RADIUS * Math.sin(angle) });
  });
  const pairs = new Set(edges.map((e) => `${e.from}>${e.to}`));
  const ended = [agents.ended, toolsHeld.ended, eventsHeld.ended].filter((e): e is string => e !== null);

  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.world.title}</h1>
        {ended.map((e) => (
          <p key={e} className="muted">{e}</p>
        ))}
        {ordered.length === 0 ? (
          <Empty title={t.world.empty} />
        ) : (
          <svg className="world" viewBox={`0 0 ${SIZE} ${SIZE}`} role="img" aria-label={t.world.title}>
            <defs>
              <marker id="world-arrow" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
                <path d="M0 0 L10 5 L0 10 z" className="world-arrow" />
              </marker>
            </defs>
            {edges.map((e) => {
              const a = at.get(e.from)!;
              const b = at.get(e.to)!;
              const dx = b.x - a.x;
              const dy = b.y - a.y;
              const length = Math.hypot(dx, dy) || 1;
              // Both directions drawn: each line shifted to its own side.
              const shift = pairs.has(`${e.to}>${e.from}`) ? 5 : 0;
              const ox = (-dy / length) * shift;
              const oy = (dx / length) * shift;
              const trim = NODE + 3;
              const x1 = a.x + (dx / length) * trim + ox;
              const y1 = a.y + (dy / length) * trim + oy;
              const x2 = b.x - (dx / length) * trim + ox;
              const y2 = b.y - (dy / length) * trim + oy;
              return (
                <g key={e.id}>
                  <line className="world-edge" x1={x1} y1={y1} x2={x2} y2={y2} markerEnd="url(#world-arrow)" />
                  <text className="world-edge-label" x={(x1 + x2) / 2 + ox * 2} y={(y1 + y2) / 2 + oy * 2} textAnchor="middle">
                    {e.label}
                  </text>
                </g>
              );
            })}
            {ordered.map((n) => {
              const p = at.get(n.key)!;
              const right = p.x >= CENTER;
              return (
                <g key={n.key} className={n.kind === "client" ? "world-node world-client" : "world-node"}>
                  {n.kind === "agent" ? (
                    <circle cx={p.x} cy={p.y} r={NODE} />
                  ) : n.kind === "tool" ? (
                    <rect x={p.x - NODE} y={p.y - NODE} width={NODE * 2} height={NODE * 2} />
                  ) : (
                    <path d={`M${p.x} ${p.y - NODE - 2} L${p.x + NODE + 2} ${p.y} L${p.x} ${p.y + NODE + 2} L${p.x - NODE - 2} ${p.y} z`} />
                  )}
                  <text className="world-label" x={p.x + (right ? NODE + 6 : -NODE - 6)} y={p.y + 4} textAnchor={right ? "start" : "end"}>
                    {n.label}
                  </text>
                </g>
              );
            })}
          </svg>
        )}
      </div>
    </div>
  );
}

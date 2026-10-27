import { daemon } from "@diverge/sdk";
import { useMemo } from "react";
import { Empty } from "../components/ui";
import { useShared } from "../lib/context";
import { type ListSpec, useAccounts, useListing } from "../lib/daemon";
import { t } from "../strings";
import { Canvas } from "../world/Canvas";
import { type Entity, type EntityKind, type Link, layout } from "../world/layout";
import { Links } from "../world/Links";
import { type Loaded, useTileset } from "../world/tileset";
import { NotConnected } from "./NotConnected";

// The daemon as a world: the accounts, agents and tools are entities on
// islands of a tile grid, and the events between them are links drawn
// over it. The tileset is the one you picked (world/tileset.ts); where
// everything goes is world/layout.ts.

const tools = daemon.endpoints.standalone_tools.list;
const dependencies = daemon.endpoints.dependency_tools.list;
const events = daemon.endpoints.events.list;
type Tool = daemon.endpoints.standalone_tools.list.server.response.Tool;
type Dependency = daemon.endpoints.dependency_tools.list.server.response.Listed;
type Event = daemon.endpoints.events.Event;
type Party = daemon.endpoints.events.Party;

const TOOLS: ListSpec<Tool, daemon.endpoints.standalone_tools.list.server.response.Frame> = {
  open: (h) => tools.client.execute(h, {}),
  key: tools.server.response.key,
  change: tools.server.response.change,
};

const DEPENDENCIES: ListSpec<Dependency, daemon.endpoints.dependency_tools.list.server.response.Frame> = {
  open: (h) => dependencies.client.execute(h, {}),
  key: dependencies.server.response.key,
  change: dependencies.server.response.change,
};

const EVENTS: ListSpec<Event, daemon.endpoints.events.list.server.response.Frame> = {
  open: (h) => events.client.execute(h, {}),
  key: daemon.endpoints.events.key,
  change: events.server.response.change,
};

type Node = { key: string; kind: EntityKind; label: string };

/** A template and an index, as a label: the template's first eight characters. */
const short = (template: string, index: number) => `${template.slice(0, 8)} #${index}`;

/**
 * The node a party of an event is: its key, its kind and its label. A
 * listed agent or tool is set first, with its name; this is the node
 * for a party no list holds.
 */
function nodeOf(party: Party): Node {
  switch (party.kind) {
    case "account":
      return { key: `account:${party.issuer}:${party.subject}`, kind: "account", label: party.subject };
    case "agent":
      return { key: `agent:${party.template}:${party.index}`, kind: "agent", label: short(party.template, party.index) };
    case "standalone_tool":
      return { key: `standalone_tool:${party.template}:${party.index}`, kind: "standalone_tool", label: short(party.template, party.index) };
    case "dependency_tool":
      return { key: `dependency_tool:${party.template}:${party.index}`, kind: "dependency_tool", label: short(party.template, party.index) };
  }
}

/** The node a listed tool is: a connected tool, another daemon's, is drawn as a standalone tool. */
function toolNode(tool: Tool): Node {
  const o = tool.origin;
  if (o.kind === "created") return { key: `standalone_tool:${o.template}:${o.index}`, kind: "standalone_tool", label: tool.name ?? short(o.template, o.index) };
  return { key: `connected:${o.daemon}:${o.index}`, kind: "standalone_tool", label: tool.name ?? `${o.daemon} #${o.index}` };
}

/** How many screen pixels each of the atlas's pixels takes. */
const SCALE = 2;

export function World() {
  const { connected, agents } = useShared();
  const accountsHeld = useAccounts(connected);
  const toolsHeld = useListing(TOOLS, connected);
  const dependenciesHeld = useListing(DEPENDENCIES, connected);
  const eventsHeld = useListing(EVENTS, connected);
  const tileset = useTileset();
  if (!connected) return <NotConnected title={t.world.title} />;

  // Every account, agent and tool listed is on the world, linked or not;
  // an event's party no list holds is added from the event.
  const nodes = new Map<string, Node>();
  for (const a of accountsHeld.items) {
    const key = `account:${a.identity.issuer}:${a.identity.subject}`;
    nodes.set(key, { key, kind: "account", label: a.identity.subject });
  }
  for (const a of agents.items) nodes.set(`agent:${a.template}:${a.index}`, { key: `agent:${a.template}:${a.index}`, kind: "agent", label: a.name ?? short(a.template, a.index) });
  for (const tool of toolsHeld.items) {
    const n = toolNode(tool);
    nodes.set(n.key, n);
  }
  for (const d of dependenciesHeld.items) {
    const key = `dependency_tool:${d.id}:${d.index}`;
    nodes.set(key, { key, kind: "dependency_tool", label: short(d.id, d.index) });
  }
  const links: Link[] = eventsHeld.items.map((e) => {
    const from = nodeOf(e.sender);
    const to = nodeOf(e.receiver);
    if (!nodes.has(from.key)) nodes.set(from.key, from);
    if (!nodes.has(to.key)) nodes.set(to.key, to);
    return { a: from.key, b: to.key };
  });
  const entities: Entity[] = [...nodes.values()].map(({ key, kind }) => ({ key, kind }));
  const names = new Map([...nodes.values()].map((n) => [n.key, n.label]));
  const ended = [accountsHeld.ended, agents.ended, toolsHeld.ended, dependenciesHeld.ended, eventsHeld.ended].filter((e): e is string => e !== null);

  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{t.world.title}</h1>
        {ended.map((e) => (
          <p key={e} className="muted">{e}</p>
        ))}
        {entities.length === 0 ? (
          <Empty title={t.world.empty} />
        ) : tileset.status === "failed" ? (
          <Empty title={t.world.noTileset} body={tileset.error} />
        ) : tileset.status === "loading" ? null : (
          <WorldStage entities={entities} links={links} names={names} loaded={tileset.loaded} />
        )}
      </div>
    </div>
  );
}

/** The grid and the links over it, as one stage that scrolls. */
function WorldStage(props: { entities: Entity[]; links: Link[]; names: Map<string, string>; loaded: Loaded }) {
  // Laid out again only when what is on it changes, not on every frame a list sends.
  const signature = JSON.stringify([props.entities, props.links]);
  const grid = useMemo(() => layout(props.entities, props.links), [signature]);
  return (
    <div className="world-stage" role="img" aria-label={t.world.title}>
      <Canvas grid={grid} loaded={props.loaded} scale={SCALE} />
      <Links grid={grid} links={props.links} names={props.names} cell={props.loaded.tileset.atlas.cell} scale={SCALE} />
    </div>
  );
}

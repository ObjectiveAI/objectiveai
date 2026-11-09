// The daemon as it stands now: the eleven lists the world is drawn
// from, each read once to its `listed` and no further, and the names
// of the agents' and tools' images.

import { daemon, shared } from "@diverge/sdk";
import { handle } from "../lib/wire";
import { imageInfo } from "./names";

type Account = daemon.endpoints.accounts.list.server.response.Account;
type Agent = daemon.endpoints.agents.list.server.response.Agent;
type Tool = daemon.endpoints.standalone_tools.list.server.response.Tool;
type ConnectedTool = daemon.endpoints.connected_standalone_tools.list.server.response.ConnectedStandaloneTool;
type Template = daemon.endpoints.standalone_tools.templates.list.server.response.Listed;
type AgentTemplate = daemon.endpoints.agents.templates.list.server.response.Listed;
type Dependency = daemon.endpoints.dependency_tools.list.server.response.Listed;
type Event = daemon.endpoints.events.Event;
type Volume = daemon.endpoints.volumes.list.server.response.Volume;
type Provider = daemon.endpoints.providers.list.server.response.Provider;
type Daemon = daemon.endpoints.daemons.list.server.response.Daemon;

/**
 * The eleven lists, the names of those the account may not read, and
 * what each tool's image calls itself, by digest (see `imageNames`).
 */
export type Snapshot = {
  accounts: Account[];
  agents: Agent[];
  tools: Tool[];
  /** The connected tools: other daemons' standalone tools, held here. */
  connectedTools: ConnectedTool[];
  templates: Template[];
  /** The agent templates: what each agent runs. */
  agentTemplates: AgentTemplate[];
  dependencies: Dependency[];
  events: Event[];
  volumes: Volume[];
  /** The providers: the records the daemon keeps of those it works with, and where it stands with each. */
  providers: Provider[];
  /** The daemons: the records the daemon keeps of our accounts on other daemons, and whether it is connected to each. */
  daemons: Daemon[];
  forbidden: string[];
  names: Map<string, string>;
  /** The digests of the images whose `/DIVERGE.md` declares a frontend. */
  frontends: Set<string>;
};

/** How one list is opened, told apart and folded. */
type Spec<T, F> = {
  name: string;
  open: () => Promise<[AsyncIterable<F>, { cancel(): Promise<void> }]>;
  key: shared.listing.Keyed<T>;
  change: (frame: F) => shared.listing.Change<T> | null;
};

/** What one list read once came to. */
type Read<T> = { items: T[]; forbidden: boolean };

/** A list that failed, by the daemon's word. */
export class ListFailed extends Error {
  constructor(list: string) {
    super(`the daemon failed to list the ${list}`);
    this.name = "ListFailed";
  }
}

/**
 * One list, read once.
 *
 * The rules:
 *
 * 1. Every frame is folded into the listing until `listed`, at which
 *    the scope is cancelled and the listing is the answer.
 * 2. `forbidden` is an empty list that says so.
 * 3. `error` is `ListFailed`.
 * 4. A stream that finishes before `listed` is the listing as it got.
 */
async function once<T, F>(spec: Spec<T, F>): Promise<Read<T>> {
  const listing = new shared.listing.Listing<T>(spec.key);
  const [stream, cancel] = await spec.open();
  for await (const frame of stream) {
    if (frame === "forbidden") return { items: [], forbidden: true };
    const change = spec.change(frame);
    if (change === null) throw new ListFailed(spec.name);
    if (change.kind === "listed") {
      await cancel.cancel();
      break;
    }
    listing.push(change);
  }
  return { items: listing.items, forbidden: false };
}

const accounts = daemon.endpoints.accounts.list;
const agents = daemon.endpoints.agents.list;
const tools = daemon.endpoints.standalone_tools.list;
const connectedTools = daemon.endpoints.connected_standalone_tools.list;
const templates = daemon.endpoints.standalone_tools.templates.list;
const dependencies = daemon.endpoints.dependency_tools.list;
const events = daemon.endpoints.events.list;
const volumes = daemon.endpoints.volumes.list;
const agentTemplates = daemon.endpoints.agents.templates.list;
const providers = daemon.endpoints.providers.list;
const daemons = daemon.endpoints.daemons.list;

/**
 * The eleven lists, read at once side by side, then the names of the
 * images the agents and tools run: an agent's from its template, a dependency tool's from its template, a
 * standalone tool's from the template it was made from. A connected
 * tool's image is the other daemon's and is not asked.
 * Throws when the daemon cannot be reached.
 */
export async function snapshot(): Promise<Snapshot> {
  const [a, g, t, c, p, d, e, v, q, r, m] = await Promise.all([
    once({ name: "accounts", open: () => accounts.client.execute(handle, {}), key: accounts.server.response.key, change: accounts.server.response.change }),
    once({ name: "agents", open: () => agents.client.execute(handle, {}), key: agents.server.response.key, change: agents.server.response.change }),
    once({ name: "standalone tools", open: () => tools.client.execute(handle, {}), key: tools.server.response.key, change: tools.server.response.change }),
    once({ name: "connected standalone tools", open: () => connectedTools.client.execute(handle, {}), key: connectedTools.server.response.key, change: connectedTools.server.response.change }),
    once({ name: "standalone tool templates", open: () => templates.client.execute(handle, {}), key: templates.server.response.key, change: templates.server.response.change }),
    once({ name: "dependency tools", open: () => dependencies.client.execute(handle, {}), key: dependencies.server.response.key, change: dependencies.server.response.change }),
    once({ name: "events", open: () => events.client.execute(handle, {}), key: daemon.endpoints.events.key, change: events.server.response.change }),
    once({ name: "volumes", open: () => volumes.client.execute(handle, {}), key: volumes.server.response.key, change: volumes.server.response.change }),
    once({ name: "agent templates", open: () => agentTemplates.client.execute(handle, {}), key: agentTemplates.server.response.key, change: agentTemplates.server.response.change }),
    once({ name: "providers", open: () => providers.client.execute(handle, {}), key: providers.server.response.key, change: providers.server.response.change }),
    once({ name: "daemons", open: () => daemons.client.execute(handle, {}), key: daemons.server.response.key, change: daemons.server.response.change }),
  ]);
  const named: [string, Read<unknown>][] = [["accounts", a], ["agents", g], ["standalone tools", t], ["connected standalone tools", c], ["standalone tool templates", p], ["dependency tools", d], ["events", e], ["volumes", v], ["agent templates", q], ["providers", r], ["daemons", m]];
  const byId = new Map(p.items.map((listed) => [listed.id, listed.template.image]));
  const images = [
    ...q.items.map((listed) => listed.template.image),
    ...d.items.map((listed) => listed.template.image),
    ...t.items.flatMap((tool) => {
      const image = byId.get(tool.template);
      return image ? [image] : [];
    }),
  ];
  const info = await imageInfo(images);
  return {
    accounts: a.items,
    agents: g.items,
    tools: t.items,
    connectedTools: c.items,
    templates: p.items,
    agentTemplates: q.items,
    dependencies: d.items,
    events: e.items,
    volumes: v.items,
    providers: r.items,
    daemons: m.items,
    forbidden: named.filter(([, read]) => read.forbidden).map(([name]) => name),
    names: new Map([...info].map(([digest, image]) => [digest, image.name])),
    frontends: new Set([...info].filter(([, image]) => image.frontend).map(([digest]) => digest)),
  };
}

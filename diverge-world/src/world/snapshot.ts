// The daemon as it stands now: the six lists the world is drawn
// from, each read once to its `listed` and no further, and the names
// of the tools' images.

import { daemon, shared } from "@diverge/sdk";
import { handle } from "../lib/wire";
import { imageNames } from "./names";

type Account = daemon.endpoints.accounts.list.server.response.Account;
type Agent = daemon.endpoints.agents.list.server.response.Agent;
type Tool = daemon.endpoints.standalone_tools.list.server.response.Tool;
type Template = daemon.endpoints.standalone_tools.templates.list.server.response.Listed;
type Dependency = daemon.endpoints.dependency_tools.list.server.response.Listed;
type Event = daemon.endpoints.events.Event;

/**
 * The six lists, the names of those the account may not read, and
 * what each tool's image calls itself, by digest (see `imageNames`).
 */
export type Snapshot = {
  accounts: Account[];
  agents: Agent[];
  tools: Tool[];
  templates: Template[];
  dependencies: Dependency[];
  events: Event[];
  forbidden: string[];
  names: Map<string, string>;
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
const templates = daemon.endpoints.standalone_tools.templates.list;
const dependencies = daemon.endpoints.dependency_tools.list;
const events = daemon.endpoints.events.list;

/**
 * The six lists, read at once side by side, then the names of the
 * images the tools run: a dependency tool's from its template, a
 * created standalone tool's from the template it was made from. A
 * connected tool's image is the other daemon's and is not asked.
 * Throws when the daemon cannot be reached.
 */
export async function snapshot(): Promise<Snapshot> {
  const [a, g, t, p, d, e] = await Promise.all([
    once({ name: "accounts", open: () => accounts.client.execute(handle, {}), key: accounts.server.response.key, change: accounts.server.response.change }),
    once({ name: "agents", open: () => agents.client.execute(handle, {}), key: agents.server.response.key, change: agents.server.response.change }),
    once({ name: "standalone tools", open: () => tools.client.execute(handle, {}), key: tools.server.response.key, change: tools.server.response.change }),
    once({ name: "standalone tool templates", open: () => templates.client.execute(handle, {}), key: templates.server.response.key, change: templates.server.response.change }),
    once({ name: "dependency tools", open: () => dependencies.client.execute(handle, {}), key: dependencies.server.response.key, change: dependencies.server.response.change }),
    once({ name: "events", open: () => events.client.execute(handle, {}), key: daemon.endpoints.events.key, change: events.server.response.change }),
  ]);
  const named: [string, Read<unknown>][] = [["accounts", a], ["agents", g], ["standalone tools", t], ["standalone tool templates", p], ["dependency tools", d], ["events", e]];
  const byId = new Map(p.items.map((listed) => [listed.id, listed.template.image]));
  const images = [
    ...d.items.map((listed) => listed.template.image),
    ...t.items.flatMap((tool) => {
      const image = tool.origin.kind === "created" ? byId.get(tool.origin.template) : undefined;
      return image ? [image] : [];
    }),
  ];
  return {
    accounts: a.items,
    agents: g.items,
    tools: t.items,
    templates: p.items,
    dependencies: d.items,
    events: e.items,
    forbidden: named.filter(([, read]) => read.forbidden).map(([name]) => name),
    names: await imageNames(images),
  };
}

// A daemon followed — ours, or one along a route: the eleven lists the
// world is drawn from, kept open for as long as the board is up, and
// the names of the agents' and tools' images as each is first seen.

import { daemon, shared } from "@diverge/sdk";
import { handle, type Route, relayed } from "../lib/wire";
import { imageInfo } from "./names";
import type { Snapshot } from "./snapshot";
import type { Watching } from "./watch";

type Event = daemon.endpoints.events.Event;
type Image = shared.containers.request.Image;

/** How long, in milliseconds, before a list that ended is opened again. */
const AGAIN = 5000;

/** How one list is opened, told apart and read. */
type Spec<T, F> = {
  name: string;
  open: () => Promise<[AsyncIterable<F>, { cancel(): Promise<void> }]>;
  key: shared.listing.Keyed<T>;
  change: (frame: F) => shared.listing.Change<T> | null;
};

/** One list kept: what it holds, whether it has been whole once, and whether the account may read it. */
type Kept<T> = { listing: shared.listing.Listing<T>; listed: boolean; forbidden: boolean };

const e = daemon.endpoints;

/**
 * Follow the daemon `route` reaches: `changed` is handed the world as
 * it stands, and `event` every change of the events list.
 *
 * The rules:
 *
 * 1. The eleven lists — accounts, agents, agent templates, standalone
 *    tools, connected standalone tools, standalone tool templates,
 *    dependency tools, events, volumes, providers, daemons — are
 *    each kept
 *    open and folded into a listing. A list that ends, fails or is
 *    refused is opened again `AGAIN` milliseconds later, until `stop`.
 * 2. A list opened again tells every item it holds once more, and
 *    never what was removed while it was closed: the items it does not
 *    tell again before its `listed` are taken out then.
 * 3. A list the account may not read is empty, and named among the
 *    `forbidden`.
 * 4. Every image an agent or a tool runs — an agent template's, a
 *    dependency tool's template's, a standalone tool's template's —
 *    never a connected tool's, the other daemon's — is asked its name once,
 *    and whether it serves a frontend, the first time it is seen
 *    (`imageInfo`); an answer is one more
 *    change.
 * 5. `changed` is first told once every list has been whole once, and
 *    then after every change, no more than once a frame, so a burst is
 *    one rebuild.
 * 6. `event` is told every change of the events list after the world
 *    that holds it was handed on, so a pulse finds its pair drawn.
 * 7. Every request — each list, each image asked — is served along
 *    `route` (`relayed`): by our own daemon for `[]`, its frames carrying
 *    no `relay`; else by the daemon at the route's end, through each
 *    before it.
 */
export function follow(route: Route, changed: (world: Snapshot) => void, event: (change: shared.listing.Change<Event>) => void): Watching {
  let stopped = false;
  const cancels = new Set<{ cancel(): Promise<void> }>();
  const names = new Map<string, string>();
  const frontends = new Set<string>();
  const asked = new Set<string>();
  const waiting: shared.listing.Change<Event>[] = [];
  let frame = 0;

  const kept = <T>(key: shared.listing.Keyed<T>): Kept<T> => ({ listing: new shared.listing.Listing<T>(key), listed: false, forbidden: false });
  const accounts = kept(e.accounts.list.server.response.key);
  const agents = kept(e.agents.list.server.response.key);
  const agentTemplates = kept(e.agents.templates.list.server.response.key);
  const tools = kept(e.standalone_tools.list.server.response.key);
  const connectedTools = kept(e.connected_standalone_tools.list.server.response.key);
  const templates = kept(e.standalone_tools.templates.list.server.response.key);
  const dependencies = kept(e.dependency_tools.list.server.response.key);
  const events = kept(e.events.key);
  const volumes = kept(e.volumes.list.server.response.key);
  const providers = kept(e.providers.list.server.response.key);
  const daemons = kept(e.daemons.list.server.response.key);
  const all: [string, { listed: boolean; forbidden: boolean }][] = [
    ["accounts", accounts],
    ["agents", agents],
    ["agent templates", agentTemplates],
    ["standalone tools", tools],
    ["connected standalone tools", connectedTools],
    ["standalone tool templates", templates],
    ["dependency tools", dependencies],
    ["events", events],
    ["volumes", volumes],
    ["providers", providers],
    ["daemons", daemons],
  ];

  /** The world as it stands. */
  const world = (): Snapshot => ({
    accounts: [...accounts.listing.items],
    agents: [...agents.listing.items],
    tools: [...tools.listing.items],
    connectedTools: [...connectedTools.listing.items],
    templates: [...templates.listing.items],
    agentTemplates: [...agentTemplates.listing.items],
    dependencies: [...dependencies.listing.items],
    events: [...events.listing.items],
    volumes: [...volumes.listing.items],
    providers: [...providers.listing.items],
    daemons: [...daemons.listing.items],
    forbidden: all.filter(([, list]) => list.forbidden).map(([name]) => name),
    names: new Map(names),
    frontends: new Set(frontends),
  });

  /** Rule 4: the images not yet asked, asked. */
  const ask = () => {
    const byId = new Map(templates.listing.items.map((listed) => [listed.id, listed.template.image]));
    const images: Image[] = [
      ...agentTemplates.listing.items.map((listed) => listed.template.image),
      ...dependencies.listing.items.map((listed) => listed.template.image),
      ...tools.listing.items.flatMap((tool) => {
        const image = byId.get(tool.template);
        return image ? [image] : [];
      }),
    ].filter((image) => !asked.has(image.digest));
    if (images.length === 0) return;
    for (const image of images) asked.add(image.digest);
    void imageInfo(images, route).then((found) => {
      if (stopped || found.size === 0) return;
      for (const [digest, image] of found) {
        names.set(digest, image.name);
        if (image.frontend) frontends.add(digest);
      }
      soon();
    });
  };

  /** Rules 5 and 6: one rebuild on the next frame, then the events it waited for. */
  const soon = () => {
    if (frame !== 0 || stopped) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      if (stopped || !all.every(([, list]) => list.listed)) return;
      ask();
      changed(world());
      for (const change of waiting.splice(0)) event(change);
    });
  };

  /** Rules 1 to 3: one list, kept open. */
  const keep = <T, F>(spec: Spec<T, F>, list: Kept<T>, tell?: (change: shared.listing.Change<T>) => void) => {
    void (async () => {
      while (!stopped) {
        let cancel: { cancel(): Promise<void> } | null = null;
        try {
          const [stream, c] = await spec.open();
          cancel = c;
          cancels.add(c);
          // Rule 2: what this opening tells before its `listed`.
          let told: shared.listing.Key[] | null = [];
          for await (const frame of stream) {
            if (stopped) break;
            if (frame === "forbidden") {
              // Rule 3.
              list.listing.items = [];
              list.forbidden = true;
              list.listed = true;
              soon();
              break;
            }
            const change = spec.change(frame);
            if (change === null) break;
            if (change.kind === "listed") {
              if (told !== null) {
                const keys = told;
                list.listing.items = list.listing.items.filter((item) => keys.some((key) => shared.listing.sameKey(key, spec.key(item))));
                told = null;
              }
              list.forbidden = false;
              list.listed = true;
              soon();
              continue;
            }
            if (told !== null && change.kind !== "removed") told.push(spec.key(change.item));
            list.listing.push(change);
            tell?.(change);
            soon();
          }
        } catch {
          // Rule 1: opened again below.
        }
        if (cancel) cancels.delete(cancel);
        if (!stopped) await new Promise((wake) => setTimeout(wake, AGAIN));
      }
    })();
  };

  // Rule 7: every list along the route.
  keep({ name: "accounts", open: () => e.accounts.list.client.execute(handle, { ...relayed(route) }), key: e.accounts.list.server.response.key, change: e.accounts.list.server.response.change }, accounts);
  keep({ name: "agents", open: () => e.agents.list.client.execute(handle, { ...relayed(route) }), key: e.agents.list.server.response.key, change: e.agents.list.server.response.change }, agents);
  keep(
    { name: "agent templates", open: () => e.agents.templates.list.client.execute(handle, { ...relayed(route) }), key: e.agents.templates.list.server.response.key, change: e.agents.templates.list.server.response.change },
    agentTemplates,
  );
  keep({ name: "standalone tools", open: () => e.standalone_tools.list.client.execute(handle, { ...relayed(route) }), key: e.standalone_tools.list.server.response.key, change: e.standalone_tools.list.server.response.change }, tools);
  keep(
    { name: "connected standalone tools", open: () => e.connected_standalone_tools.list.client.execute(handle, { ...relayed(route) }), key: e.connected_standalone_tools.list.server.response.key, change: e.connected_standalone_tools.list.server.response.change },
    connectedTools,
  );
  keep(
    { name: "standalone tool templates", open: () => e.standalone_tools.templates.list.client.execute(handle, { ...relayed(route) }), key: e.standalone_tools.templates.list.server.response.key, change: e.standalone_tools.templates.list.server.response.change },
    templates,
  );
  keep({ name: "dependency tools", open: () => e.dependency_tools.list.client.execute(handle, { ...relayed(route) }), key: e.dependency_tools.list.server.response.key, change: e.dependency_tools.list.server.response.change }, dependencies);
  keep({ name: "events", open: () => e.events.list.client.execute(handle, { ...relayed(route) }), key: e.events.key, change: e.events.list.server.response.change }, events, (change) => {
    // Rule 6, once the list was whole: what it told before is the baseline, not a pulse.
    if (events.listed) waiting.push(change);
  });
  keep({ name: "volumes", open: () => e.volumes.list.client.execute(handle, { ...relayed(route) }), key: e.volumes.list.server.response.key, change: e.volumes.list.server.response.change }, volumes);
  keep({ name: "providers", open: () => e.providers.list.client.execute(handle, { ...relayed(route) }), key: e.providers.list.server.response.key, change: e.providers.list.server.response.change }, providers);
  keep({ name: "daemons", open: () => e.daemons.list.client.execute(handle, { ...relayed(route) }), key: e.daemons.list.server.response.key, change: e.daemons.list.server.response.change }, daemons);

  return {
    stop() {
      stopped = true;
      if (frame !== 0) cancelAnimationFrame(frame);
      for (const cancel of cancels) void cancel.cancel();
    },
  };
}

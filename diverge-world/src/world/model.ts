// What the world shows: every account, agent, standalone tool and
// dependency tool, and who has sent to whom.

import type { daemon } from "@diverge/sdk";
import type { Snapshot } from "./snapshot";

/** The four kinds of entity. */
export type Kind = "account" | "agent" | "standalone_tool" | "dependency_tool";

/** One entity: its key, its kind, the name on its plate, and what its hover says. */
export type Entity = { key: string; kind: Kind; label: string; detail: string };

/**
 * Two entities joined: `a` before `b` by key, and which ways something
 * has passed between them — `ab` from `a` to `b`, `ba` from `b` to `a`.
 */
export type Pair = { a: string; b: string; ab: boolean; ba: boolean };

/** The world as entities and the pairs among them. */
export type Model = { entities: Entity[]; pairs: Pair[] };

type Party = daemon.endpoints.events.Party;
type Tool = daemon.endpoints.standalone_tools.list.server.response.Tool;

/** What a tool is named: its image's name, if the image said one, else the fallback. */
const named = (snapshot: Snapshot, digest: string | undefined, fallback: string) =>
  (digest === undefined ? undefined : snapshot.names.get(digest)) ?? fallback;

/** A template and an index, as a label: the template's first eight characters. */
const short = (template: string, index: number) => `${template.slice(0, 8)} #${index}`;

/** The entity an event's party is, for a party no list holds. */
function partyEntity(party: Party): Entity {
  switch (party.kind) {
    case "account":
      return { key: `account:${party.issuer}:${party.subject}`, kind: "account", label: party.subject, detail: `${party.subject} at ${party.issuer}` };
    case "agent":
      return { key: `agent:${party.template}:${party.index}`, kind: "agent", label: short(party.template, party.index), detail: `${party.template} #${party.index}` };
    case "standalone_tool":
      return { key: `standalone_tool:${party.template}:${party.index}`, kind: "standalone_tool", label: short(party.template, party.index), detail: `${party.template} #${party.index}` };
    case "dependency_tool":
      return { key: `dependency_tool:${party.template}:${party.index}`, kind: "dependency_tool", label: short(party.template, party.index), detail: `${party.template} #${party.index}` };
  }
}

/**
 * The entity a listed tool is: a connected tool, another daemon's, is
 * drawn as a standalone tool. A created tool is named by its image,
 * else by the name its create gave it, else by its template and index.
 */
function toolEntity(snapshot: Snapshot, tool: Tool): Entity {
  const o = tool.origin;
  if (o.kind === "created") {
    const image = snapshot.templates.find((listed) => listed.id === o.template)?.template.image;
    const label = named(snapshot, image?.digest, tool.name ?? short(o.template, o.index));
    return { key: `standalone_tool:${o.template}:${o.index}`, kind: "standalone_tool", label, detail: `${label} — ${o.template} #${o.index}` };
  }
  const label = tool.name ?? `${o.daemon} #${o.index}`;
  return { key: `connected:${o.daemon}:${o.index}`, kind: "standalone_tool", label, detail: `${label} — connected from ${o.daemon}` };
}

/**
 * The model a snapshot makes.
 *
 * The rules:
 *
 * 1. Every listed account, agent, standalone tool and dependency tool
 *    is an entity, linked or not. A tool is named by its image's name
 *    (`imageNames`) where the image said one.
 * 2. An event's party that no list holds is an entity too, made from
 *    the event.
 * 3. Every event joins its sender to its receiver, that way. An event
 *    whose sender is its receiver joins nothing.
 * 4. All the events between two entities, either way, are one pair,
 *    which says which ways they ran.
 */
export function model(snapshot: Snapshot): Model {
  const entities = new Map<string, Entity>();
  const put = (e: Entity) => {
    if (!entities.has(e.key)) entities.set(e.key, e);
  };
  // Rule 1.
  for (const a of snapshot.accounts) {
    const { issuer, subject } = a.identity;
    put({ key: `account:${issuer}:${subject}`, kind: "account", label: subject, detail: `${subject} at ${issuer}` });
  }
  for (const a of snapshot.agents) {
    const label = a.name ?? short(a.template, a.index);
    put({ key: `agent:${a.template}:${a.index}`, kind: "agent", label, detail: `${label} — ${a.template} #${a.index}` });
  }
  for (const tool of snapshot.tools) put(toolEntity(snapshot, tool));
  for (const d of snapshot.dependencies) {
    const label = named(snapshot, d.template.image.digest, short(d.id, d.index));
    put({ key: `dependency_tool:${d.id}:${d.index}`, kind: "dependency_tool", label, detail: `${label} — ${d.id} #${d.index}` });
  }
  // Rules 2 to 4.
  const pairs = new Map<string, Pair>();
  for (const event of snapshot.events) {
    const from = partyEntity(event.sender);
    const to = partyEntity(event.receiver);
    put(from);
    put(to);
    if (from.key === to.key) continue;
    const [a, b] = from.key < to.key ? [from.key, to.key] : [to.key, from.key];
    const id = `${a}\n${b}`;
    const pair = pairs.get(id) ?? { a, b, ab: false, ba: false };
    if (from.key === a) pair.ab = true;
    else pair.ba = true;
    pairs.set(id, pair);
  }
  return { entities: [...entities.values()], pairs: [...pairs.values()] };
}

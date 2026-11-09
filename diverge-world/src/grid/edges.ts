// The edges: every line the board can draw between two nodes, of three
// types — calls, attachments and creators.

import type { daemon } from "@diverge/sdk";
import { accountKey, connectedKey, dependencyKey, type Model, providerAt, providerKey, toolKey, volumeKey } from "../world/model";
import type { Snapshot } from "../world/snapshot";

/** The three types of line, in the order their lanes lie. */
export type EdgeType = "call" | "attachment" | "creator";
export const TYPES: EdgeType[] = ["call", "attachment", "creator"];

/**
 * One line between two nodes: `a` before `b` by key, the pair's fixed
 * frame; its type; and, a call's or a creator's, which end it runs
 * from — the sender, the maker.
 */
export type Edge = { a: string; b: string; type: EdgeType; from?: string };

type Owner = daemon.endpoints.dependency_tools.list.server.response.Listed["owner"];
type Creator = daemon.creator.Creator;
type ToolRef = daemon.key.Tool;

/** An agent's key, by its template and index, as `model` makes it. */
const agentAt = (agent: { template: string; index: number }) => `agent:${agent.template}:${agent.index}`;

/** An owner as one string: the agent, the agent template, or `global`; an agent's name left off. */
function ownerId(owner: Owner): string {
  if (owner === "global") return "global";
  if ("agent" in owner) return agentAt(owner.agent);
  return `template:${owner.agent_template.template}`;
}

/** Two keys in the pair's order. */
const ordered = (x: string, y: string): [string, string] => (x < y ? [x, y] : [y, x]);

/**
 * Every edge a snapshot holds, both of whose ends are on the board.
 *
 * The rules:
 *
 * 1. A call is every way an event pair ran (`model`'s pairs), from its
 *    sender. A mount is not a call.
 * 2. An attachment joins, once per pair and without a way:
 *    a. a standalone tool or a connected standalone tool and every
 *       agent it is attached to directly, and every listed agent of
 *       every agent template it is attached to;
 *    b. a dependency tool and every agent that depends on it;
 *    c. a volume and every agent and tool that mounts it;
 *    d. a provider and what it runs: every agent and standalone tool
 *       `deployed` whose `provider` names it, and every dependency tool
 *       whose run's provider names it — a run only while deployed. A
 *       provider's last deployment, its container down, draws nothing.
 *    e. a provider and every volume it holds.
 * 3. A creator runs from the maker to what it made, for every agent,
 *    standalone tool, connected standalone tool, dependency tool,
 *    account and provider whose `creator` is on
 *    the board: the client's account, the agent, the created tool by
 *    its template and index, the dependency tool by its template and
 *    owner. A maker deleted or unlisted draws nothing.
 * 4. An edge whose two ends are one node, or with an end not on the
 *    board, is left out.
 */
export function edges(snapshot: Snapshot, made: Model): Edge[] {
  const on = new Set(made.entities.map((e) => e.key));
  const out: Edge[] = [];
  const seen = new Set<string>();
  const add = (x: string | undefined, y: string | undefined, type: EdgeType, from?: string) => {
    // Rule 4.
    if (x === undefined || y === undefined || x === y || !on.has(x) || !on.has(y)) return;
    const [a, b] = ordered(x, y);
    const id = `${type}\n${a}\n${b}\n${from ?? ""}`;
    if (seen.has(id)) return;
    seen.add(id);
    out.push(from === undefined ? { a, b, type } : { a, b, type, from });
  };
  const dependencyAt = new Map(snapshot.dependencies.map((d) => [`${d.id}\n${ownerId(d.owner)}`, dependencyKey(d)]));
  const toolAt = (tool: ToolRef): string | undefined => {
    if ("dependency" in tool) return dependencyAt.get(`${tool.dependency.template}\n${ownerId(tool.dependency.owner)}`);
    if ("connected_standalone" in tool) return connectedKey(tool.connected_standalone);
    return toolKey(tool.record);
  };
  // Rule 1.
  for (const p of made.pairs) {
    if (p.mount) continue;
    if (p.ab) add(p.a, p.b, "call", p.a);
    if (p.ba) add(p.a, p.b, "call", p.b);
  }
  // Rule 2a.
  const attached = (key: string, t: { attached_agent_instances?: { template: string; index: number }[]; attached_agent_templates?: string[] }) => {
    for (const agent of t.attached_agent_instances ?? []) add(key, agentAt(agent), "attachment");
    for (const template of t.attached_agent_templates ?? []) {
      for (const agent of snapshot.agents) if (agent.template === template) add(key, agentAt(agent), "attachment");
    }
  };
  for (const t of snapshot.tools) attached(toolKey(t), t);
  for (const t of snapshot.connectedTools) attached(connectedKey(t), t);
  // Rule 2b.
  for (const d of snapshot.dependencies) for (const x of d.agents ?? []) add(dependencyKey(d), agentAt(x.agent), "attachment");
  // Rule 2c.
  for (const v of snapshot.volumes) {
    for (const agent of v.agents ?? []) add(volumeKey(v), agentAt(agent), "attachment");
    for (const tool of v.tools ?? []) add(volumeKey(v), toolAt(tool), "attachment");
  }
  // Rule 2d.
  for (const a of snapshot.agents) if (a.deployed && a.provider) add(providerAt(a.provider.identity), agentAt(a), "attachment");
  for (const t of snapshot.tools) if (t.deployed && t.provider) add(providerAt(t.provider.identity), toolKey(t), "attachment");
  for (const d of snapshot.dependencies) if (d.run) add(providerAt(d.run.provider.identity), dependencyKey(d), "attachment");
  // Rule 2e.
  for (const v of snapshot.volumes) add(providerAt(v.provider), volumeKey(v), "attachment");
  // Rule 3.
  const maker = (c: Creator | undefined): string | undefined => {
    if (c === undefined) return undefined;
    if (c.type === "client") return `account:${c.identity.issuer}:${c.identity.subject}`;
    if (c.type === "agent") return agentAt(c);
    if ("record" in c) return `standalone_tool:${c.record.template}:${c.record.index}`;
    return dependencyAt.get(`${c.dependency.template}\n${ownerId(c.dependency.owner)}`);
  };
  const madeBy = (key: string, c: Creator | undefined) => {
    const m = maker(c);
    add(m, key, "creator", m);
  };
  for (const a of snapshot.agents) madeBy(agentAt(a), a.creator);
  for (const t of snapshot.tools) madeBy(toolKey(t), t.creator);
  for (const t of snapshot.connectedTools) madeBy(connectedKey(t), t.creator);
  for (const d of snapshot.dependencies) madeBy(dependencyKey(d), d.creator);
  for (const a of snapshot.accounts) madeBy(accountKey(a), a.creator);
  for (const p of snapshot.providers) madeBy(providerKey(p), p.creator);
  return out;
}

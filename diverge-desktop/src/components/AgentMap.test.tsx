import { readFileSync } from "node:fs";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { AgentView } from "../bindings/AgentView";
import type { MapNode } from "../bindings/MapNode";
import { SharedContext, type Shared } from "../lib/context";
import { AgentsMap } from "../screens/AgentsMap";
import { t } from "../strings";
import { AgentMap } from "./AgentMap";

const agent = (key: string, over: Partial<AgentView> = {}): AgentView => ({
  key, name: key, template: "t", index: 1, creator: { kind: "client", identity: "you" }, tags: [], image_name: "diverge-agentic-loop-cc", digest: "",
  created: "2026-10-15T08:00:00Z", active: false, last_active: null, provider: null, logs_index: 0, tools: [], ...over,
});
const node = (a: AgentView, children: MapNode[] = [], cards = 0): MapNode => ({ key: `agent:${a.key}`, what: { kind: "agent", agent: a, cards }, working: a.active, children });
const site = agent("site-fixes", { active: true, last_active: "2026-10-15T08:00:00Z", provider: { kind: "outgoing", address: "127.0.0.1:4640" }, tools: ["nightly-digest"], tags: ["website"] });
const links = agent("links", { last_active: "2026-10-15T07:00:00Z" });
const digest = agent("digest");
const tree: MapNode[] = [
  node(site, [node(links)], 2),
  { key: "tool:tt#1", what: { kind: "tool", maker: { kind: "tool", template: "tt", index: 1, name: "nightly-digest" } }, working: false, children: [node(digest)] },
];
const shared = (over: Partial<Shared>) => ({ agents: [site, links, digest], agentsState: { state: "live" }, tree, cards: [], withdrawn: [], knocks: [], spaces: [], info: null, homeId: null, open: () => {}, close: () => {}, ...over }) as unknown as Shared;
const render = (s: Shared) => renderToStaticMarkup(createElement(SharedContext.Provider, { value: s }, createElement(AgentsMap)));

describe("the Agents map by keyboard", () => {
  const html = render(shared({}));
  it("is one tree that Tab enters once, at one card; arrow keys reach the rest", () => {
    expect(html.match(/role="tree"/g)).toHaveLength(1);
    const stops = [...html.matchAll(/role="treeitem"[^>]*tabindex="(-?\d)"/g)].map((m) => m[1]);
    expect(stops).toEqual(["0", "-1", "-1", "-1"]);
    // Each card says where it is in the tree, and what it made hangs in a group it owns.
    expect([...html.matchAll(/aria-level="(\d)"/g)].map((m) => m[1])).toEqual(["1", "2", "1", "2"]);
    expect(html).toContain('aria-owns="map-under-agent:site-fixes"');
    expect(html).toContain('role="group" id="map-under-agent:site-fixes"');
  });

  it("puts every control on every card in the Tab order", () => {
    expect(html).not.toMatch(/<button[^>]*tabindex="-1"/);
    // Each agent's card opens its conversation with a button of its own, named for it.
    for (const a of [site, links, digest]) expect(html).toContain(`title="${t.map.open} ${a.name}"`);
  });

  it("names each card for a screen reader: working or not, and what waits on you", () => {
    expect(html).toContain(`aria-label="site-fixes, ${t.status.working}, 2 ${t.map.waitingOnYou}"`);
    expect(html).toContain(`aria-label="links, ${t.status.idle}"`);
    expect(html).toContain(`aria-label="nightly-digest"`);
  });
});

describe("the Agents map's layout", () => {
  it("grows down by default, and across when asked, with the same cards", () => {
    const down = renderToStaticMarkup(createElement(AgentMap, { tree, layout: "down", onOpen: () => {} }));
    const across = renderToStaticMarkup(createElement(AgentMap, { tree, layout: "across", onOpen: () => {} }));
    expect(down).toMatch(/class="map-canvas map-down"/);
    expect(across).toMatch(/class="map-canvas map-across"/);
    expect(down.replace("map-down", "")).toBe(across.replace("map-across", ""));
  });

  it("draws a stem and a rail to each thing an agent made only where there is one, and a rail capped at the ends", () => {
    const html = renderToStaticMarkup(createElement(AgentMap, { tree: [node(site, [node(links), node(digest)])], layout: "down", onOpen: () => {} }));
    expect(html.match(/class="map-stem"/g)).toHaveLength(3);
    expect(html).toContain('class="map-rail-lead map-rail-off"');
    expect(html).toContain('class="map-rail-trail map-rail-off"');
  });

  it("draws an agent made solid and a helper dashed, in the theme's edge colour", () => {
    const css = readFileSync(new URL("../app.css", import.meta.url), "utf8");
    expect(css).toMatch(/\.map-stem \{[^}]*border: 0 solid var\(--line-strong\)/);
    expect(css).toMatch(/\.map-helpers \{[^}]*border-left: 1px dashed var\(--line-strong\)/);
    expect(css).not.toMatch(/\.map[\w-]* \{[^}]*zoom/);
  });

  it("says when there are no agents yet, with the way to make one", () => {
    const html = render(shared({ agents: [], tree: [] }));
    expect(html).toContain(t.map.emptyTitle);
    expect(html).toContain(t.rail.newAgent);
    expect(html).not.toContain('role="tree"');
  });
});

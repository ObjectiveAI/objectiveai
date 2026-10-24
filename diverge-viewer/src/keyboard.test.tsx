import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Rail } from "./components/Rail";
import { SharedContext, type Shared } from "./lib/context";
import type { Agent } from "./lib/daemon";
import { t } from "./strings";

// Agents the rail is handed, and no longer lists.
const ada = { template: "t1", index: 1, name: "ada", active: true } as Agent;
const second = { template: "t1", index: 2, active: false } as Agent;
const shared: Shared = {
  agents: { items: [ada, second], listed: true, ended: null },
  connected: true,
  link: { state: "connected" },
  redial: () => {},
  info: null,
  open: () => {},
  close: () => {},
};
const render = (el: ReturnType<typeof createElement>) => renderToStaticMarkup(createElement(SharedContext.Provider, { value: shared }, el));

describe("the rail", () => {
  const html = render(createElement(Rail, { focused: null }));
  it("lists its places in order, Graph first, and no agent", () => {
    const places = [t.rail.graph, t.rail.agents, t.rail.tools, t.rail.storage, t.rail.views, t.rail.settings].map((name) => html.indexOf(`>${name}<`));
    expect(places.every((at) => at >= 0)).toBe(true);
    expect([...places].sort((a, b) => a - b)).toEqual(places);
    expect(html).not.toContain(">ada<");
  });
});

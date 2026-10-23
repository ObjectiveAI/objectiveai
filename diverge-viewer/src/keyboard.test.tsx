import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Rail } from "./components/Rail";
import { SharedContext, type Shared } from "./lib/context";
import type { Agent } from "./lib/daemon";
import { t } from "./strings";

// Only what the rail reads of an agent; the rest the daemon sends.
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

describe("an agent in the rail", () => {
  const html = render(createElement(Rail, { focused: null }));
  it("is named, and one with no name is named by its number", () => {
    expect(html).toContain(">ada<");
    expect(html).toContain(`>${t.rail.unnamed} 2<`);
  });
});

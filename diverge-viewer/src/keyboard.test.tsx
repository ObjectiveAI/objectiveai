import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { TabsSnapshot } from "./bindings/TabsSnapshot";
import { Rail } from "./components/Rail";
import { TabStrip } from "./components/TabStrip";
import { SharedContext, type Shared } from "./lib/context";
import { t } from "./strings";

const shared: Shared = {
  agents: [{ template: "t1", index: 1, name: "ada", active: true, lastActive: null }, { template: "t1", index: 2, name: null, active: false, lastActive: null }],
  connected: true,
  info: null,
  open: () => {},
  close: () => {},
};
const render = (el: ReturnType<typeof createElement>) => renderToStaticMarkup(createElement(SharedContext.Provider, { value: shared }, el));

describe("the tab strip by keyboard", () => {
  const snapshot: TabsSnapshot = {
    generation: 1,
    focused: "agent:t1:1",
    tabs: [{ key: "settings", tab: { kind: "settings" } }, { key: "agent:t1:1", tab: { kind: "agent", template: "t1", index: 1, name: "ada" } }, { key: "storage", tab: { kind: "storage" } }],
  };
  const html = render(createElement(TabStrip, { snapshot, onFocus: () => {}, onClose: () => {} }));
  it("Tab enters the strip once, at the tab you're on; the rest are reached with arrow keys", () => {
    const stops = [...html.matchAll(/role="tab" tabindex="(-?\d)"/g)].map((m) => m[1]);
    expect(stops).toEqual(["-1", "0", "-1"]);
  });
  it("each tab's close button names the tab it closes", () => {
    expect(html).toContain(`aria-label="${t.common.close} ${t.settings.title}"`);
    expect(html).toContain(`aria-label="${t.common.close} ada"`);
  });
});

describe("an agent in the rail", () => {
  const html = render(createElement(Rail, { focused: null }));
  it("is named, and one with no name is named by its number", () => {
    expect(html).toContain(">ada<");
    expect(html).toContain(`>${t.rail.unnamed} 2<`);
  });
});

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { SpaceSummary } from "./bindings/SpaceSummary";
import type { TabsSnapshot } from "./bindings/TabsSnapshot";
import { Rail } from "./components/Rail";
import { TabStrip } from "./components/TabStrip";
import { SharedContext, type Shared } from "./lib/context";
import { t } from "./strings";

const room: SpaceSummary = {
  id: "room-1", title: "Saturday Workshop for anyone who fixes things", kind: "board", host: { kind: "outgoing", address: "203.0.113.7:7000" }, host_name: "juno",
  mine: true, online: true, you_are: "juno", you_key: "k", you_account: null, fresh: false,
};
const shared = { agents: [], listedAt: 0, info: null, spaces: [room], homeId: null, knocks: [], cards: [], withdrawn: [], open: () => {}, close: () => {} } as unknown as Shared;
const render = (el: ReturnType<typeof createElement>) => renderToStaticMarkup(createElement(SharedContext.Provider, { value: shared }, el));

describe("the tab strip by keyboard", () => {
  const snapshot: TabsSnapshot = { generation: 1, focused: "inbox", tabs: [{ key: "home", tab: { kind: "home" } }, { key: "inbox", tab: { kind: "inbox" } }, { key: "space:room-1", tab: { kind: "space", id: "room-1" } }] };
  const html = render(createElement(TabStrip, { snapshot, onFocus: () => {}, onClose: () => {} }));
  it("Tab enters the strip once, at the tab you're on; the rest are reached with arrow keys", () => {
    const stops = [...html.matchAll(/role="tab" tabindex="(-?\d)"/g)].map((m) => m[1]);
    expect(stops).toEqual(["-1", "0", "-1"]);
  });
  it("each tab's close button names the tab it closes", () => {
    expect(html).toContain(`aria-label="${t.common.close} ${t.inbox.title}"`);
    expect(html).toContain(`aria-label="${t.common.close} ${room.title}"`);
  });
});

describe("a room in the rail", () => {
  const html = render(createElement(Rail, { focused: null }));
  it("shows its whole name, with what kind of room it is in the row's title instead of beside it", () => {
    const row = html.match(/<button[^>]*title="([^"]*)"[^>]*>(?:(?!<\/button>).)*Saturday Workshop(?:(?!<\/button>).)*<\/button>/);
    expect(row?.[1]).toContain(t.spaces.kinds.board);
    expect(row?.[0]).not.toContain(`>${t.spaces.kinds.board}<`);
  });
});

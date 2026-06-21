// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach } from "vitest";
import { listen, __resetForTests } from "./index";

/**
 * Simulate the iframe context by mocking `window.parent` to be a
 * distinct object that proxies postMessage to our test harness. The
 * SDK's `isInIframe()` check is just `window.parent !== window`.
 */
function setupIframeContext() {
  __resetForTests();
  const parentMessages: unknown[] = [];
  const parent = {
    postMessage: (msg: unknown) => parentMessages.push(msg),
  };
  // Override window.parent for the duration of the test.
  Object.defineProperty(window, "parent", { value: parent, configurable: true });
  return {
    parentMessages,
    /** Simulate a message arriving from the parent. */
    deliver(msg: unknown) {
      window.dispatchEvent(new MessageEvent("message", { data: msg }));
    },
  };
}

function teardownIframeContext() {
  Object.defineProperty(window, "parent", { value: window, configurable: true });
  __resetForTests();
}

describe("listen in iframe context", () => {
  let ctx: ReturnType<typeof setupIframeContext>;
  beforeEach(() => {
    ctx = setupIframeContext();
  });
  afterEach(teardownIframeContext);

  it("fires the handler when a matching inbound event arrives", () => {
    const calls: unknown[] = [];
    listen<{ x: number }>("my_event", (v) => calls.push(v));
    ctx.deliver({
      kind: "plugin-event",
      type: "inbound",
      sub_type: "my_event",
      value: { x: 42 },
    });
    expect(calls).toEqual([{ x: 42 } as unknown]);
  });

  it("ignores inbound events with other sub_types", () => {
    const calls: unknown[] = [];
    listen("my_event", (v) => calls.push(v));
    ctx.deliver({
      kind: "plugin-event",
      type: "inbound",
      sub_type: "other_event",
      value: 1,
    });
    expect(calls).toEqual([]);
  });

  it("ignores cli_command events (they go through ViewerCommandExecutor, not listen)", () => {
    const calls: unknown[] = [];
    listen("my_event", (v) => calls.push(v));
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      value: { type: "notification" },
    });
    expect(calls).toEqual([]);
  });

  it("returns an unlisten that stops further events", () => {
    const calls: unknown[] = [];
    const unlisten = listen("my_event", (v) => calls.push(v));
    ctx.deliver({
      kind: "plugin-event",
      type: "inbound",
      sub_type: "my_event",
      value: 1,
    });
    unlisten();
    ctx.deliver({
      kind: "plugin-event",
      type: "inbound",
      sub_type: "my_event",
      value: 2,
    });
    expect(calls).toEqual([1]);
  });
});

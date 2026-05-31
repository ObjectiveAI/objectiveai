// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach } from "vitest";
import { listen, invokeCli, __resetForTests } from "./index";

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

  it("ignores cli_command events (they go through invokeCli, not listen)", () => {
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

describe("invokeCli in iframe context", () => {
  let ctx: ReturnType<typeof setupIframeContext>;
  beforeEach(() => {
    ctx = setupIframeContext();
  });
  afterEach(teardownIframeContext);

  it("posts a cli-invoke message to window.parent with the args", () => {
    const iter = invokeCli(["agents", "spawn"])[Symbol.asyncIterator]();
    // Trigger the postMessage path by entering the iterator.
    void iter.next();
    expect(ctx.parentMessages).toEqual([
      { kind: "cli-invoke", args: ["agents", "spawn"] },
    ]);
  });

  it("yields each cli_command line and terminates on `{type: end}`", async () => {
    const iterable = invokeCli(["test"]);
    const collected: unknown[] = [];

    const run = (async () => {
      for await (const line of iterable) {
        collected.push(line);
      }
    })();

    // Let the iterator subscribe before delivering lines.
    await new Promise((r) => setTimeout(r, 0));

    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      value: { type: "begin" },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      value: { type: "notification", value: { hello: "world" } },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      value: { type: "end" },
    });

    await run;

    expect(collected).toEqual([
      { type: "begin" },
      { type: "notification", value: { hello: "world" } },
      { type: "end" },
    ]);
  });

  it("ignores inbound events while collecting cli output", async () => {
    const iterable = invokeCli(["test"]);
    const collected: unknown[] = [];

    const run = (async () => {
      for await (const line of iterable) {
        collected.push(line);
      }
    })();

    await new Promise((r) => setTimeout(r, 0));

    ctx.deliver({
      kind: "plugin-event",
      type: "inbound",
      sub_type: "unrelated",
      value: { ignored: true },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      value: { type: "notification", value: 1 },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      value: { type: "end" },
    });

    await run;

    expect(collected).toEqual([
      { type: "notification", value: 1 },
      { type: "end" },
    ]);
  });
});

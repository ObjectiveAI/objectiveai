// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach } from "vitest";
import { ViewerPluginExecutor } from "./viewerPlugin";
import type { CliCommandRequest } from "../request";

/**
 * Simulate the iframe context by mocking `window.parent` to be a distinct
 * object that proxies postMessage to our test harness. The executor's
 * in-iframe check is just `window.parent !== window`.
 */
function setupIframeContext() {
  const parentMessages: Array<{ kind: string; id: string; request: unknown }> =
    [];
  const parent = {
    postMessage: (msg: unknown) =>
      parentMessages.push(msg as { kind: string; id: string; request: unknown }),
  };
  Object.defineProperty(window, "parent", { value: parent, configurable: true });
  return {
    parentMessages,
    /** Simulate a message arriving from the parent. */
    deliver(msg: unknown) {
      window.dispatchEvent(new MessageEvent("message", { data: msg }));
    },
    /** The invocation id the executor minted for the n-th post. */
    idOf(n: number): string {
      return parentMessages[n].id;
    },
  };
}

function teardownIframeContext() {
  Object.defineProperty(window, "parent", { value: window, configurable: true });
}

// The transport ignores `request`'s shape (it just JSON-posts it), so the
// tests use minimal stand-ins cast to the request type.
const asRequest = (r: unknown): CliCommandRequest => r as CliCommandRequest;

describe("ViewerPluginExecutor", () => {
  it("throws when constructed outside an iframe", () => {
    // jsdom default: top-level window.
    expect(() => new ViewerPluginExecutor()).toThrow(/plugin-only/);
  });
});

describe("ViewerPluginExecutor.execute in iframe context", () => {
  let ctx: ReturnType<typeof setupIframeContext>;
  let executor: ViewerPluginExecutor;
  beforeEach(() => {
    ctx = setupIframeContext();
    executor = new ViewerPluginExecutor();
  });
  afterEach(teardownIframeContext);

  it("posts a cli-execute message with the typed request and a fresh id", () => {
    const request = asRequest({ path: "agents", command: { path: "spawn" } });
    const iter = executor.execute(request)[Symbol.asyncIterator]();
    // Trigger the postMessage path by entering the iterator.
    void iter.next();
    expect(ctx.parentMessages).toHaveLength(1);
    const posted = ctx.parentMessages[0];
    expect(posted.kind).toBe("cli-execute");
    expect(posted.request).toBe(request);
    expect(typeof posted.id).toBe("string");
    expect(posted.id.length).toBeGreaterThan(0);
  });

  it("mints a distinct id per invocation", () => {
    void executor.execute(asRequest({ path: "a" }))[Symbol.asyncIterator]().next();
    void executor.execute(asRequest({ path: "b" }))[Symbol.asyncIterator]().next();
    expect(ctx.parentMessages).toHaveLength(2);
    expect(ctx.idOf(0)).not.toBe(ctx.idOf(1));
  });

  it("yields each cli_command line for its id and terminates on `{type: end}`", async () => {
    const iterable = executor.execute(asRequest({ path: "test" }));
    const collected: unknown[] = [];

    const run = (async () => {
      for await (const line of iterable) {
        collected.push(line);
      }
    })();

    // Let the iterator subscribe (and post) before delivering lines.
    await new Promise((r) => setTimeout(r, 0));
    const id = ctx.idOf(0);

    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id,
      value: { type: "begin" },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id,
      value: { type: "notification", value: { hello: "world" } },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id,
      value: { type: "end" },
    });

    await run;

    expect(collected).toEqual([
      { type: "begin" },
      { type: "notification", value: { hello: "world" } },
      { type: "end" },
    ]);
  });

  it("demuxes concurrent invocations by id", async () => {
    const first = executor.execute(asRequest({ path: "first" }));
    const second = executor.execute(asRequest({ path: "second" }));
    const firstLines: unknown[] = [];
    const secondLines: unknown[] = [];

    const runs = Promise.all([
      (async () => {
        for await (const line of first) firstLines.push(line);
      })(),
      (async () => {
        for await (const line of second) secondLines.push(line);
      })(),
    ]);

    await new Promise((r) => setTimeout(r, 0));
    const firstId = ctx.idOf(0);
    const secondId = ctx.idOf(1);

    // Interleave the two runs' lines.
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id: firstId,
      value: { n: 1 },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id: secondId,
      value: { n: 2 },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id: secondId,
      value: { type: "end" },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id: firstId,
      value: { n: 3 },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id: firstId,
      value: { type: "end" },
    });

    await runs;

    expect(firstLines).toEqual([{ n: 1 }, { n: 3 }, { type: "end" }]);
    expect(secondLines).toEqual([{ n: 2 }, { type: "end" }]);
  });

  it("ignores unrelated events and other ids while collecting", async () => {
    const iterable = executor.execute(asRequest({ path: "test" }));
    const collected: unknown[] = [];

    const run = (async () => {
      for await (const line of iterable) {
        collected.push(line);
      }
    })();

    await new Promise((r) => setTimeout(r, 0));
    const id = ctx.idOf(0);

    ctx.deliver({
      kind: "plugin-event",
      type: "inbound",
      value: { ignored: true },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id: "someone-else",
      value: { stolen: true },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id,
      value: { type: "notification", value: 1 },
    });
    ctx.deliver({
      kind: "plugin-event",
      type: "cli_command",
      id,
      value: { type: "end" },
    });

    await run;

    expect(collected).toEqual([
      { type: "notification", value: 1 },
      { type: "end" },
    ]);
  });
});

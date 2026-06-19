// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach } from "vitest";
import { ViewerCommandExecutor } from "./viewer";
import type { CliCommandRequest } from "../request";

/**
 * Simulate the iframe context by mocking `window.parent` to be a distinct
 * object that proxies postMessage to our test harness. The executor's
 * in-iframe check is just `window.parent !== window`.
 */
function setupIframeContext() {
  const parentMessages: unknown[] = [];
  const parent = {
    postMessage: (msg: unknown) => parentMessages.push(msg),
  };
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
}

// The transport ignores `request`'s shape (it just JSON-posts it), so the
// tests use minimal stand-ins cast to the request type.
const asRequest = (r: unknown): CliCommandRequest => r as CliCommandRequest;

describe("ViewerCommandExecutor.execute in iframe context", () => {
  let ctx: ReturnType<typeof setupIframeContext>;
  let executor: ViewerCommandExecutor;
  beforeEach(() => {
    ctx = setupIframeContext();
    executor = new ViewerCommandExecutor();
  });
  afterEach(teardownIframeContext);

  it("posts a cli-execute message to window.parent with the typed request", () => {
    const request = asRequest({ path: "agents", command: { path: "spawn" } });
    const iter = executor.execute(request)[Symbol.asyncIterator]();
    // Trigger the postMessage path by entering the iterator.
    void iter.next();
    expect(ctx.parentMessages).toEqual([{ kind: "cli-execute", request }]);
  });

  it("yields each cli_command line and terminates on `{type: end}`", async () => {
    const iterable = executor.execute(asRequest({ path: "test" }));
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
    const iterable = executor.execute(asRequest({ path: "test" }));
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

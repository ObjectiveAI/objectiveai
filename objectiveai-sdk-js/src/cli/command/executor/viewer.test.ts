// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import { ViewerCommandExecutor } from "./viewer";
import type { CliCommandRequest } from "../request";

// ── @tauri-apps/api mock: the main-viewer transport's host side ─────

const host = vi.hoisted(() => {
  const listeners: Array<(event: { payload: unknown }) => void> = [];
  const invokes: Array<{ cmd: string; args: Record<string, unknown> }> = [];
  return {
    listeners,
    invokes,
    /** Whether the next invoke call rejects (transport failure). */
    rejectInvokes: false,
    emit(payload: unknown): void {
      for (const handler of [...listeners]) {
        handler({ payload });
      }
    },
    reset(): void {
      listeners.length = 0;
      invokes.length = 0;
      host.rejectInvokes = false;
    },
  };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: async (
    _channel: string,
    handler: (event: { payload: unknown }) => void,
  ): Promise<() => void> => {
    host.listeners.push(handler);
    return () => {
      const i = host.listeners.indexOf(handler);
      if (i !== -1) host.listeners.splice(i, 1);
    };
  },
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string, args: Record<string, unknown>) => {
    if (host.rejectInvokes) throw new Error("invoke failed");
    host.invokes.push({ cmd, args });
    return undefined;
  },
}));

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

/** Let the main-viewer transport's async setup (dynamic imports +
 * channel subscription + invoke) settle. */
async function settle() {
  await new Promise((r) => setTimeout(r, 0));
}

// The transport ignores `request`'s shape (it just JSON-posts it), so the
// tests use minimal stand-ins cast to the request type.
const asRequest = (r: unknown): CliCommandRequest => r as CliCommandRequest;

describe("ViewerCommandExecutor.execute in iframe context", () => {
  let ctx: ReturnType<typeof setupIframeContext>;
  let executor: ViewerCommandExecutor;
  beforeEach(() => {
    ctx = setupIframeContext();
    host.reset();
    executor = new ViewerCommandExecutor();
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

  it("never touches the Tauri transport", () => {
    void executor.execute(asRequest({ path: "a" }))[Symbol.asyncIterator]().next();
    expect(host.invokes).toEqual([]);
    expect(host.listeners).toEqual([]);
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
    await settle();
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

    await settle();
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

    await settle();
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

describe("ViewerCommandExecutor.execute in main viewer context", () => {
  let executor: ViewerCommandExecutor;
  beforeEach(() => {
    // jsdom's default: top-level window, window.parent === window.
    host.reset();
    executor = new ViewerCommandExecutor();
  });

  /** The invocation id the executor minted for the n-th invoke. */
  function idOf(n: number): string {
    return host.invokes[n].args.id as string;
  }

  it("subscribes to the objectiveai channel, then invokes cli_execute with a fresh id and the objectiveai destination", async () => {
    const request = asRequest({ path: "agents", command: { path: "spawn" } });
    void executor.execute(request)[Symbol.asyncIterator]().next();
    await settle();

    // Subscription happened (and stays attached mid-run).
    expect(host.listeners).toHaveLength(1);
    expect(host.invokes).toHaveLength(1);
    const { cmd, args } = host.invokes[0];
    expect(cmd).toBe("cli_execute");
    expect(args.request).toBe(request);
    expect(args.destination).toBe("objectiveai");
    expect(typeof args.id).toBe("string");
    expect((args.id as string).length).toBeGreaterThan(0);
  });

  it("yields each cli_command line for its id and terminates on `{type: end}`", async () => {
    const iterable = executor.execute(asRequest({ path: "test" }));
    const collected: unknown[] = [];

    const run = (async () => {
      for await (const line of iterable) {
        collected.push(line);
      }
    })();

    await settle();
    const id = idOf(0);

    host.emit({ type: "cli_command", id, value: { type: "begin" } });
    host.emit({
      type: "cli_command",
      id,
      value: { type: "notification", value: { hello: "world" } },
    });
    host.emit({ type: "cli_command", id, value: { type: "end" } });

    await run;

    expect(collected).toEqual([
      { type: "begin" },
      { type: "notification", value: { hello: "world" } },
      { type: "end" },
    ]);
    // The channel subscription is released once the stream ends.
    expect(host.listeners).toHaveLength(0);
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

    await settle();
    const firstId = idOf(0);
    const secondId = idOf(1);
    expect(firstId).not.toBe(secondId);

    // Interleave the two runs' lines.
    host.emit({ type: "cli_command", id: firstId, value: { n: 1 } });
    host.emit({ type: "cli_command", id: secondId, value: { n: 2 } });
    host.emit({ type: "cli_command", id: secondId, value: { type: "end" } });
    host.emit({ type: "cli_command", id: firstId, value: { n: 3 } });
    host.emit({ type: "cli_command", id: firstId, value: { type: "end" } });

    await runs;

    expect(firstLines).toEqual([{ n: 1 }, { n: 3 }, { type: "end" }]);
    expect(secondLines).toEqual([{ n: 2 }, { type: "end" }]);
  });

  it("ignores inbound events and other ids while collecting", async () => {
    const iterable = executor.execute(asRequest({ path: "test" }));
    const collected: unknown[] = [];

    const run = (async () => {
      for await (const line of iterable) {
        collected.push(line);
      }
    })();

    await settle();
    const id = idOf(0);

    host.emit({ type: "inbound", value: { ignored: true } });
    host.emit({
      type: "cli_command",
      id: "someone-else",
      value: { stolen: true },
    });
    host.emit({
      type: "cli_command",
      id,
      value: { type: "notification", value: 1 },
    });
    host.emit({ type: "cli_command", id, value: { type: "end" } });

    await run;

    expect(collected).toEqual([
      { type: "notification", value: 1 },
      { type: "end" },
    ]);
  });

  it("surfaces an invoke rejection as one error line, then terminates", async () => {
    host.rejectInvokes = true;
    const collected: unknown[] = [];
    for await (const line of executor.execute(asRequest({ path: "test" }))) {
      collected.push(line);
    }

    expect(collected).toEqual([
      {
        type: "error",
        level: "error",
        fatal: null,
        message: "Error: invoke failed",
      },
      { type: "end" },
    ]);
  });
});

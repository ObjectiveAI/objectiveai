// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";

/**
 * Tests for the autonomous daemon-listener singleton: the
 * daemon_config connect, the typed execution-handler registry (root
 * callbacks per path, nested item callbacks over one shared drain),
 * error isolation, the built-in plugins/run forwarding, and the
 * reconnect loop. All infra is mocked — the SDK listener, the tauri
 * invoke, and the bridge delivery.
 */

// ── mocks ───────────────────────────────────────────────────────────

const harness = vi.hoisted(() => {
  type FakeListener = {
    executions: unknown[];
    wake: (() => void) | null;
    ended: boolean;
    push(execution: unknown): void;
    end(): void;
  };
  const h = {
    daemonConfig: {
      address: "ws://127.0.0.1:4242",
      signature: "sha256=abc",
    } as { address: string; signature: string | null } | null,
    connects: [] as Array<{ url: string; options: unknown }>,
    listeners: [] as FakeListener[],
    delivered: [] as Array<{ coords: unknown; frame: unknown }>,
    newListener(): FakeListener {
      const listener: FakeListener = {
        executions: [],
        wake: null,
        ended: false,
        push(execution: unknown) {
          listener.executions.push(execution);
          listener.wake?.();
        },
        end() {
          listener.ended = true;
          listener.wake?.();
        },
      };
      h.listeners.push(listener);
      return listener;
    },
    reset() {
      h.daemonConfig = {
        address: "ws://127.0.0.1:4242",
        signature: "sha256=abc",
      };
      h.connects.length = 0;
      h.listeners.length = 0;
      h.delivered.length = 0;
    },
  };
  return h;
});

vi.mock("./lib/tauri", () => ({
  tauriInvoke: async (cmd: string) => {
    if (cmd !== "daemon_config") throw new Error(`unexpected invoke: ${cmd}`);
    return harness.daemonConfig ?? undefined;
  },
}));

vi.mock("./plugin-bridge", () => ({
  deliverInbound: (coords: unknown, frame: unknown) => {
    harness.delivered.push({ coords, frame });
  },
}));

vi.mock("@objectiveai/sdk", () => ({
  WebSocketListener: {
    connect: async (url: string, options: unknown) => {
      harness.connects.push({ url, options });
      const listener = harness.newListener();
      return {
        async *[Symbol.asyncIterator]() {
          for (;;) {
            while (listener.executions.length > 0) {
              yield listener.executions.shift();
            }
            if (listener.ended) return;
            await new Promise<void>((resolve) => {
              listener.wake = resolve;
            });
            listener.wake = null;
          }
        },
      };
    },
  },
}));

// ── fixtures ────────────────────────────────────────────────────────

/** A controllable live-only response stream. Counts its iterator
 * subscriptions so tests can assert the shared single drain. */
function fakeResponse() {
  const pending: unknown[] = [];
  let done = false;
  let wake: (() => void) | null = null;
  const stream = {
    subscriptions: 0,
    push(item: unknown) {
      pending.push(item);
      wake?.();
    },
    end() {
      done = true;
      wake?.();
    },
    async *[Symbol.asyncIterator]() {
      stream.subscriptions += 1;
      for (;;) {
        while (pending.length > 0) yield pending.shift();
        if (done) return;
        await new Promise<void>((resolve) => {
          wake = resolve;
        });
        wake = null;
      }
    },
  };
  return stream;
}

function execution(
  pathType: string,
  extra: Record<string, unknown>,
  response: unknown,
) {
  return {
    request: { path_type: pathType, ...extra },
    agentArguments: { agent_id: "agent-a" },
    response,
  };
}

async function settle() {
  await new Promise((r) => setTimeout(r, 0));
}

type PathType = import("./daemon-listener").PathType;

describe("daemon-listener registry", () => {
  let mod: typeof import("./daemon-listener");

  beforeEach(async () => {
    vi.resetModules();
    harness.reset();
    mod = await import("./daemon-listener");
  });

  it("connects to the config-derived /listen URL with the signature", async () => {
    mod.startDaemonListener();
    await settle();
    expect(harness.connects).toEqual([
      {
        url: "ws://127.0.0.1:4242/listen",
        options: { signature: "sha256=abc" },
      },
    ]);
  });

  it("is idempotent — one connection no matter how often started", async () => {
    mod.startDaemonListener();
    mod.startDaemonListener();
    mod.startDaemonListener();
    await settle();
    expect(harness.connects).toHaveLength(1);
  });

  it("stays idle without daemon_config (browser dev)", async () => {
    harness.daemonConfig = null;
    mod.startDaemonListener();
    await settle();
    expect(harness.connects).toHaveLength(0);
  });

  it("fires the registered root handler per matching execution, before any item", async () => {
    const seen: unknown[] = [];
    mod.registerExecutionHandler("agents/list" as PathType, (e) => {
      seen.push(e);
    });
    mod.startDaemonListener();
    await settle();

    const listed = execution("agents/list", {}, fakeResponse());
    const other = execution("swarms/list", {}, fakeResponse());
    harness.listeners[0].push(listed);
    harness.listeners[0].push(other);
    await settle();

    expect(seen).toEqual([listed]);
  });

  it("drives a returned bare-function callback over each item, and drains once for many handlers", async () => {
    const first: unknown[] = [];
    const second: unknown[] = [];
    let ended = 0;
    mod.registerExecutionHandler("agents/list" as PathType, () => {
      return (item: unknown) => first.push(item);
    });
    mod.registerExecutionHandler("agents/list" as PathType, () => ({
      onItem: (item: unknown) => second.push(item),
      onEnd: () => {
        ended += 1;
      },
    }));
    mod.startDaemonListener();
    await settle();

    const response = fakeResponse();
    harness.listeners[0].push(execution("agents/list", {}, response));
    await settle();
    response.push({ n: 1 });
    response.push({ n: 2 });
    response.end();
    await settle();

    expect(first).toEqual([{ n: 1 }, { n: 2 }]);
    expect(second).toEqual([{ n: 1 }, { n: 2 }]);
    expect(ended).toBe(1);
    // One shared drain — never one iterator per handler.
    expect(response.subscriptions).toBe(1);
  });

  it("normalizes unary Promise responses to one item + end", async () => {
    const items: unknown[] = [];
    let ended = 0;
    mod.registerExecutionHandler("agents/get" as PathType, () => ({
      onItem: (item: unknown) => items.push(item),
      onEnd: () => {
        ended += 1;
      },
    }));
    mod.startDaemonListener();
    await settle();

    harness.listeners[0].push(
      execution("agents/get", {}, Promise.resolve({ agent: "a" })),
    );
    await settle();

    expect(items).toEqual([{ agent: "a" }]);
    expect(ended).toBe(1);
  });

  it("does not drain an execution nobody attached to", async () => {
    mod.registerExecutionHandler("agents/list" as PathType, () => {
      // Observed the envelope, declined the items.
    });
    mod.startDaemonListener();
    await settle();

    const response = fakeResponse();
    harness.listeners[0].push(execution("agents/list", {}, response));
    // An execution with NO handler at all is also untouched.
    const unhandled = fakeResponse();
    harness.listeners[0].push(execution("swarms/list", {}, unhandled));
    await settle();

    expect(response.subscriptions).toBe(0);
    expect(unhandled.subscriptions).toBe(0);
  });

  it("isolates throwing handlers and item callbacks", async () => {
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    const survived: unknown[] = [];
    mod.registerExecutionHandler("agents/list" as PathType, () => {
      throw new Error("root boom");
    });
    mod.registerExecutionHandler("agents/list" as PathType, () => ({
      onItem: () => {
        throw new Error("item boom");
      },
    }));
    mod.registerExecutionHandler("agents/list" as PathType, () => ({
      onItem: (item: unknown) => survived.push(item),
    }));
    mod.startDaemonListener();
    await settle();

    const response = fakeResponse();
    harness.listeners[0].push(execution("agents/list", {}, response));
    await settle();
    response.push({ n: 1 });
    response.end();
    await settle();

    expect(survived).toEqual([{ n: 1 }]);
    expect(errors).toHaveBeenCalled();
    errors.mockRestore();
  });

  it("unregister detaches the handler", async () => {
    const seen: unknown[] = [];
    const unregister = mod.registerExecutionHandler(
      "agents/list" as PathType,
      (e) => {
        seen.push(e);
      },
    );
    mod.startDaemonListener();
    await settle();

    unregister();
    harness.listeners[0].push(execution("agents/list", {}, fakeResponse()));
    await settle();
    expect(seen).toEqual([]);
  });

  it("forwards plugins/run executions as the three-frame envelope with one minted id", async () => {
    mod.startDaemonListener();
    await settle();

    const response = fakeResponse();
    harness.listeners[0].push(
      execution(
        "plugins/run",
        { owner: "objectiveai", name: "alpha", version: "0.0.1", args: [] },
        response,
      ),
    );
    await settle();
    response.push({ hello: "world" });
    response.push({ type: "mcp", url: "http://x" });
    response.end();
    await settle();

    const frames = harness.delivered;
    expect(frames).toHaveLength(4);
    const coords = { owner: "objectiveai", name: "alpha", version: "0.0.1" };
    for (const d of frames) expect(d.coords).toEqual(coords);

    const request = frames[0].frame as Record<string, unknown>;
    expect(request.agent_id).toBe("agent-a");
    const id = request.id as string;
    expect(typeof id).toBe("string");
    expect(id.length).toBeGreaterThan(0);
    expect(frames[1].frame).toEqual({ id, value: { hello: "world" } });
    expect(frames[2].frame).toEqual({ id, value: { type: "mcp", url: "http://x" } });
    expect(frames[3].frame).toEqual({ id, end: true });
  });

  it("keeps registrations across reconnects", async () => {
    vi.useFakeTimers();
    try {
      const seen: unknown[] = [];
      mod.registerExecutionHandler("agents/list" as PathType, (e) => {
        seen.push(e);
      });
      mod.startDaemonListener();
      await vi.advanceTimersByTimeAsync(0);
      expect(harness.connects).toHaveLength(1);

      harness.listeners[0].end();
      await vi.advanceTimersByTimeAsync(1000);
      expect(harness.connects).toHaveLength(2);

      const listed = execution("agents/list", {}, fakeResponse());
      harness.listeners[1].push(listed);
      await vi.advanceTimersByTimeAsync(0);
      expect(seen).toEqual([listed]);
    } finally {
      vi.useRealTimers();
    }
  });
});

// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";

/**
 * Tests for the singleton daemon-listener wrapper: the daemon_config
 * connect, in-app fan-out via daemonRuns(), plugins/run forwarding
 * through the bridge's deliverInbound, and the reconnect loop. All
 * infra is mocked — the SDK listener, the tauri invoke, and the
 * bridge delivery.
 */

// ── mocks ───────────────────────────────────────────────────────────

const harness = vi.hoisted(() => {
  type FakeListener = {
    runs: unknown[];
    wake: (() => void) | null;
    ended: boolean;
    push(run: unknown): void;
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
        runs: [],
        wake: null,
        ended: false,
        push(run: unknown) {
          listener.runs.push(run);
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
            while (listener.runs.length > 0) {
              yield listener.runs.shift();
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

/** A minimal live-only response stream stand-in (subscribe-then-push). */
function fakeResponseStream() {
  const pending: unknown[] = [];
  let done = false;
  let wake: (() => void) | null = null;
  return {
    push(item: unknown) {
      pending.push(item);
      wake?.();
    },
    end() {
      done = true;
      wake?.();
    },
    async *[Symbol.asyncIterator]() {
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
}

function pluginsRun(name: string) {
  const response = fakeResponseStream();
  return {
    run: {
      request: {
        path_type: "plugins/run",
        owner: "objectiveai",
        name,
        version: "0.0.1",
        args: [],
      },
      agentArguments: { agent_id: "agent-a" },
      response,
    },
    response,
  };
}

async function settle() {
  await new Promise((r) => setTimeout(r, 0));
}

describe("daemon-listener", () => {
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

  it("forwards a plugins/run run as the three-frame envelope with one minted id", async () => {
    mod.startDaemonListener();
    await settle();
    const { run, response } = pluginsRun("alpha");
    harness.listeners[0].push(run);
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
    expect(request.value).toBe(run.request);
    expect(request.agent_id).toBe("agent-a");
    const id = request.id as string;
    expect(typeof id).toBe("string");
    expect(id.length).toBeGreaterThan(0);

    expect(frames[1].frame).toEqual({ id, value: { hello: "world" } });
    expect(frames[2].frame).toEqual({ id, value: { type: "mcp", url: "http://x" } });
    expect(frames[3].frame).toEqual({ id, end: true });
  });

  it("mints a distinct id per run", async () => {
    mod.startDaemonListener();
    await settle();
    const a = pluginsRun("alpha");
    const b = pluginsRun("beta");
    harness.listeners[0].push(a.run);
    harness.listeners[0].push(b.run);
    await settle();
    a.response.end();
    b.response.end();
    await settle();

    const requestFrames = harness.delivered
      .map((d) => d.frame as Record<string, unknown>)
      .filter((f) => "value" in f && (f.value as { path_type?: string })?.path_type === "plugins/run");
    expect(requestFrames).toHaveLength(2);
    expect(requestFrames[0].id).not.toBe(requestFrames[1].id);
  });

  it("routes every run to daemonRuns() subscribers; only plugins/run forwards", async () => {
    mod.startDaemonListener();
    await settle();

    const seen: unknown[] = [];
    const iterating = (async () => {
      for await (const run of mod.daemonRuns()) {
        seen.push(run);
        if (seen.length === 2) break;
      }
    })();
    await settle();

    const other = {
      request: { path_type: "agents/list" },
      agentArguments: {},
      response: fakeResponseStream(),
    };
    harness.listeners[0].push(other);
    const { run, response } = pluginsRun("alpha");
    harness.listeners[0].push(run);
    await settle();
    response.end();
    await iterating;

    expect(seen).toEqual([other, run]);
    // Only the plugins/run run produced deliveries.
    const requestFrames = harness.delivered.map(
      (d) => (d.frame as Record<string, unknown>).value,
    );
    expect(requestFrames).not.toContain(other.request);
  });

  it("reconnects after the connection ends", async () => {
    vi.useFakeTimers();
    try {
      mod.startDaemonListener();
      await vi.advanceTimersByTimeAsync(0);
      expect(harness.connects).toHaveLength(1);

      harness.listeners[0].end();
      await vi.advanceTimersByTimeAsync(1000);
      expect(harness.connects).toHaveLength(2);

      // Subscribers survive the reconnect.
      const seen: unknown[] = [];
      const iterating = (async () => {
        for await (const run of mod.daemonRuns()) {
          seen.push(run);
          break;
        }
      })();
      await vi.advanceTimersByTimeAsync(0);
      const { run, response } = pluginsRun("alpha");
      harness.listeners[1].push(run);
      await vi.advanceTimersByTimeAsync(0);
      response.end();
      await iterating;
      expect(seen).toEqual([run]);
    } finally {
      vi.useRealTimers();
    }
  });
});

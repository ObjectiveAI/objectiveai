// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";

/**
 * Behavior tests for the main-viewer `ViewerListener`: transport
 * (the host's "objectiveai" Tauri channel), frame routing by id,
 * live-only multi-subscriber delivery, and the plugin-iframe guard.
 */

// ── @tauri-apps/api/event mock: the "objectiveai" channel ──────────

const host = vi.hoisted(() => {
  const listeners: Array<(event: { payload: unknown }) => void> = [];
  return {
    listeners,
    emit(payload: unknown): void {
      for (const handler of [...listeners]) {
        handler({ payload });
      }
    },
    reset(): void {
      listeners.length = 0;
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

import { ViewerListener, ResponseItemStream } from "./viewerListener";
import type { CliCommandListenerExecution } from "../cli/command/listenerExecution";

/** One daemon broadcast frame, delivered as the host does: wrapped in
 * an `inbound`-typed event on the "objectiveai" channel. */
function inbound(frame: unknown): void {
  host.emit({ type: "inbound", destination: "objectiveai", value: frame });
}

/** Let the listener's async transport attach (dynamic import + listen). */
async function settle() {
  await new Promise((r) => setTimeout(r, 0));
}

/** Collect the next `n` runs off a fresh iterator. */
function collectRuns(
  listener: ViewerListener,
  n: number,
): Promise<CliCommandListenerExecution[]> {
  return (async () => {
    const out: CliCommandListenerExecution[] = [];
    for await (const run of listener) {
      out.push(run);
      if (out.length === n) break;
    }
    return out;
  })();
}

describe("ViewerListener", () => {
  beforeEach(() => {
    host.reset();
    ViewerListener.__resetForTests();
  });
  afterEach(() => {
    ViewerListener.__resetForTests();
  });

  it("throws when constructed inside a plugin iframe", () => {
    Object.defineProperty(window, "parent", { value: {}, configurable: true });
    try {
      expect(() => new ViewerListener()).toThrow(/main-viewer-only/);
    } finally {
      Object.defineProperty(window, "parent", {
        value: window,
        configurable: true,
      });
    }
  });

  it("is a singleton — constructing again hands back the same instance", () => {
    const a = new ViewerListener();
    const b = new ViewerListener();
    expect(b).toBe(a);
  });

  it("attaches to the objectiveai channel on construction", async () => {
    new ViewerListener();
    await settle();
    expect(host.listeners).toHaveLength(1);
  });

  it("yields an envelope per request frame and feeds its stream by id", async () => {
    const listener = new ViewerListener();
    await settle();
    const runsPromise = collectRuns(listener, 1);
    await settle();

    inbound({
      id: "run-1",
      agent_id: "agent-a",
      value: { path_type: "plugins/run", owner: "o", name: "n" },
    });

    const [run] = await runsPromise;
    expect((run.request as { path_type: string }).path_type).toBe(
      "plugins/run",
    );
    expect(
      (run.agentArguments as { agent_id?: string | null }).agent_id,
    ).toBe("agent-a");
    expect(run.response).toBeInstanceOf(ResponseItemStream);

    const items = (
      run.response as ResponseItemStream<unknown>
    ).toArray();
    await settle();
    inbound({ id: "run-1", value: { n: 1 } });
    inbound({ id: "run-1", value: { n: 2 } });
    inbound({ id: "run-1", end: true });
    expect(await items).toEqual([{ n: 1 }, { n: 2 }]);
  });

  it("settles a unary run's promise with its first response frame", async () => {
    const listener = new ViewerListener();
    await settle();
    const runsPromise = collectRuns(listener, 1);
    await settle();

    inbound({ id: "run-u", value: { path_type: "agents/enqueue" } });
    const [run] = await runsPromise;
    expect(run.response).toBeInstanceOf(Promise);

    inbound({ id: "run-u", value: { ok: true } });
    inbound({ id: "run-u", end: true });
    expect(await run.response).toEqual({ ok: true });
  });

  it("demuxes interleaved runs by id", async () => {
    const listener = new ViewerListener();
    await settle();
    const runsPromise = collectRuns(listener, 2);
    await settle();

    inbound({ id: "a", value: { path_type: "plugins/run" } });
    inbound({ id: "b", value: { path_type: "plugins/list" } });
    const [runA, runB] = await runsPromise;
    const itemsA = (runA.response as ResponseItemStream<unknown>).toArray();
    const itemsB = (runB.response as ResponseItemStream<unknown>).toArray();
    await settle();

    inbound({ id: "a", value: { from: "a1" } });
    inbound({ id: "b", value: { from: "b1" } });
    inbound({ id: "a", value: { from: "a2" } });
    inbound({ id: "b", end: true });
    inbound({ id: "a", end: true });

    expect(await itemsA).toEqual([{ from: "a1" }, { from: "a2" }]);
    expect(await itemsB).toEqual([{ from: "b1" }]);
  });

  it("skips runs whose path_type is unknown, dropping their frames", async () => {
    const listener = new ViewerListener();
    await settle();
    const runsPromise = collectRuns(listener, 1);
    await settle();

    inbound({ id: "mystery", value: { path_type: "not/a/real/path" } });
    inbound({ id: "mystery", value: { dropped: true } });
    inbound({ id: "mystery", end: true });
    inbound({ id: "real", value: { path_type: "plugins/run" } });

    const [run] = await runsPromise;
    expect((run.request as { path_type: string }).path_type).toBe(
      "plugins/run",
    );
  });

  it("ignores cli_command events on the channel", async () => {
    const listener = new ViewerListener();
    await settle();
    const runsPromise = collectRuns(listener, 1);
    await settle();

    host.emit({
      type: "cli_command",
      id: "invocation-1",
      value: { path_type: "plugins/run" },
    });
    inbound({ id: "real", value: { path_type: "plugins/run" } });

    const [run] = await runsPromise;
    expect((run.request as { path_type: string }).path_type).toBe(
      "plugins/run",
    );
  });

  it("delivers live-only: a late subscriber misses earlier runs", async () => {
    const listener = new ViewerListener();
    await settle();
    const early = collectRuns(listener, 2);
    await settle();

    inbound({ id: "first", value: { path_type: "plugins/run" } });

    const late = collectRuns(listener, 1);
    await settle();
    inbound({ id: "second", value: { path_type: "plugins/list" } });

    const earlyRuns = await early;
    const lateRuns = await late;
    expect(
      earlyRuns.map((r) => (r.request as { path_type: string }).path_type),
    ).toEqual(["plugins/run", "plugins/list"]);
    expect(
      lateRuns.map((r) => (r.request as { path_type: string }).path_type),
    ).toEqual(["plugins/list"]);
  });
});

// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach } from "vitest";
import { ViewerPluginListener, type ViewerPluginRun } from "./viewerPluginListener";
import { ResponseItemStream } from "./websocketListener";

/**
 * Behavior tests for the plugin-iframe ViewerPluginListener: the
 * inbound postMessage transport (synthetic — all infra mocked), the
 * fixed-type plugins/run routing by broadcast id, live-only
 * multi-subscriber delivery, and the plugin-only guard.
 */

/** Enter the iframe context: window.parent becomes a distinct object. */
function setupIframeContext() {
  Object.defineProperty(window, "parent", { value: {}, configurable: true });
}

function teardownIframeContext() {
  Object.defineProperty(window, "parent", { value: window, configurable: true });
}

/** Deliver one broadcast frame as the host would: wrapped in an
 * inbound plugin-event postMessage. */
function inbound(frame: unknown): void {
  window.dispatchEvent(
    new MessageEvent("message", {
      data: { kind: "plugin-event", type: "inbound", value: frame },
    }),
  );
}

/** A plugins/run request frame with context. */
function requestFrame(id: string, extra?: Record<string, unknown>) {
  return {
    id,
    agent_id: "agent-a",
    plugin_owner: "objectiveai",
    value: {
      path_type: "plugins/run",
      owner: "objectiveai",
      name: "alpha",
      version: "0.0.1",
      args: ["--hello"],
    },
    ...extra,
  };
}

/** Collect the next `n` runs off a fresh iterator. */
function collectRuns(
  listener: ViewerPluginListener,
  n: number,
): Promise<ViewerPluginRun[]> {
  return (async () => {
    const out: ViewerPluginRun[] = [];
    for await (const run of listener) {
      out.push(run);
      if (out.length === n) break;
    }
    return out;
  })();
}

async function settle() {
  await new Promise((r) => setTimeout(r, 0));
}

describe("ViewerPluginListener", () => {
  let listener: ViewerPluginListener | null = null;
  beforeEach(setupIframeContext);
  afterEach(() => {
    listener?.close();
    listener = null;
    teardownIframeContext();
  });

  it("throws when constructed outside an iframe", () => {
    teardownIframeContext();
    expect(() => new ViewerPluginListener()).toThrow(/plugin-only/);
    setupIframeContext();
  });

  it("yields a run per request frame — id preserved, request and agent arguments extracted", async () => {
    listener = new ViewerPluginListener();
    const runsPromise = collectRuns(listener, 1);

    inbound(requestFrame("run-1"));

    const [run] = await runsPromise;
    expect(run.id).toBe("run-1");
    expect(run.request.args).toEqual(["--hello"]);
    expect(run.request.name).toBe("alpha");
    expect(run.agentArguments.agent_id).toBe("agent-a");
    // plugin_* context fields are not agent arguments.
    expect(run.agentArguments).not.toHaveProperty("plugin_owner");
    expect(run.response).toBeInstanceOf(ResponseItemStream);
  });

  it("feeds the run's response stream by id and ends it on the terminator", async () => {
    listener = new ViewerPluginListener();
    const runsPromise = collectRuns(listener, 1);

    inbound(requestFrame("run-1"));
    const [run] = await runsPromise;
    const items = run.response.toArray();
    await settle();

    inbound({ id: "run-1", value: { hello: "world" } });
    inbound({
      id: "run-1",
      value: { type: "error", level: "warn", fatal: null, message: "hm" },
    });
    inbound({ id: "run-1", end: true });

    expect(await items).toEqual([
      { hello: "world" },
      { type: "error", level: "warn", fatal: null, message: "hm" },
    ]);
    expect(run.response.done).toBe(true);
  });

  it("demuxes interleaved concurrent runs by id", async () => {
    listener = new ViewerPluginListener();
    const runsPromise = collectRuns(listener, 2);

    inbound(requestFrame("a"));
    inbound(requestFrame("b"));
    const [runA, runB] = await runsPromise;
    expect([runA.id, runB.id]).toEqual(["a", "b"]);
    const itemsA = runA.response.toArray();
    const itemsB = runB.response.toArray();
    await settle();

    inbound({ id: "a", value: { from: "a1" } });
    inbound({ id: "b", value: { from: "b1" } });
    inbound({ id: "a", value: { from: "a2" } });
    inbound({ id: "b", end: true });
    inbound({ id: "a", end: true });

    expect(await itemsA).toEqual([{ from: "a1" }, { from: "a2" }]);
    expect(await itemsB).toEqual([{ from: "b1" }]);
  });

  it("ignores cli_command plugin-events and non-plugin-event messages", async () => {
    listener = new ViewerPluginListener();
    const runsPromise = collectRuns(listener, 1);

    window.dispatchEvent(
      new MessageEvent("message", {
        data: {
          kind: "plugin-event",
          type: "cli_command",
          id: "invocation-1",
          value: requestFrame("not-a-run"),
        },
      }),
    );
    window.dispatchEvent(
      new MessageEvent("message", { data: { anything: "else" } }),
    );
    window.dispatchEvent(new MessageEvent("message", { data: null }));
    inbound(requestFrame("real"));

    const [run] = await runsPromise;
    expect(run.id).toBe("real");
  });

  it("defensively skips non-plugins/run request frames and drops their later frames", async () => {
    listener = new ViewerPluginListener();
    const runsPromise = collectRuns(listener, 1);

    inbound({ id: "mystery", value: { path_type: "agents/list" } });
    inbound({ id: "mystery", value: { dropped: true } });
    inbound({ id: "mystery", end: true });
    inbound(requestFrame("real"));

    const [run] = await runsPromise;
    expect(run.id).toBe("real");
  });

  it("delivers live-only: a late subscriber misses earlier runs", async () => {
    listener = new ViewerPluginListener();
    const early = collectRuns(listener, 2);
    await settle();

    inbound(requestFrame("first"));

    const late = collectRuns(listener, 1);
    await settle();
    inbound(requestFrame("second"));

    expect((await early).map((r) => r.id)).toEqual(["first", "second"]);
    expect((await late).map((r) => r.id)).toEqual(["second"]);
  });

  it("close() ends root iterators and open streams, and detaches", async () => {
    listener = new ViewerPluginListener();
    const allRuns = (async () => {
      const out: ViewerPluginRun[] = [];
      for await (const run of listener!) out.push(run);
      return out;
    })();
    await settle();

    inbound(requestFrame("open"));
    await settle();

    listener.close();

    const runs = await allRuns;
    expect(runs.map((r) => r.id)).toEqual(["open"]);
    expect(runs[0].response.done).toBe(true);

    // Detached: frames after close() do nothing.
    inbound(requestFrame("after-close"));
    const post = listener.runs();
    expect((await post.next()).done).toBe(true);
  });
});

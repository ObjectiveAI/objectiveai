// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";

/**
 * Behavior tests for the `BroadcastListener` `/listen` SSE consumer:
 * transport (a mocked global `fetch` — nothing connects), header auth,
 * frame routing by id, live-only multi-subscriber delivery, and the
 * Rust-parity close lifecycle.
 */

const encoder = new TextEncoder();

/** A controllable stand-in for one `fetch`ed SSE response. */
class MockSseConnection {
  static instances: MockSseConnection[] = [];

  url: string;
  headers: Record<string, string>;
  aborted = false;
  stream: ReadableStream<Uint8Array>;
  #controller!: ReadableStreamDefaultController<Uint8Array>;

  constructor(url: string, init: RequestInit) {
    this.url = url;
    this.headers = (init.headers ?? {}) as Record<string, string>;
    this.stream = new ReadableStream<Uint8Array>({
      start: (controller) => {
        this.#controller = controller;
      },
      cancel: () => {
        this.aborted = true;
      },
    });
    init.signal?.addEventListener("abort", () => {
      this.aborted = true;
      try {
        this.#controller.error(new Error("aborted"));
      } catch {
        // Already closed.
      }
    });
    MockSseConnection.instances.push(this);
  }

  // ── test drivers ──
  frame(value: unknown): void {
    this.#controller.enqueue(
      encoder.encode(`data: ${JSON.stringify(value)}\n\n`),
    );
  }

  serverClose(): void {
    try {
      this.#controller.close();
    } catch {
      // Already closed.
    }
  }
}

/** Mock `fetch`: 2xx + a controllable SSE body, unless `status` says
 * otherwise. */
function stubFetch(status = 200): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init: RequestInit) => {
      if (status !== 200) {
        return { ok: false, status, body: null };
      }
      const connection = new MockSseConnection(url, init);
      return { ok: true, status, body: connection.stream };
    }),
  );
}

import { BroadcastListener, ResponseItemStream } from "./broadcastListener";
import type { CliCommandListenerExecution } from "./command/listenerExecution";

/** Connect a listener against the newest mock connection. */
async function connect(options?: {
  signature?: string | null;
}): Promise<{ listener: BroadcastListener; sse: MockSseConnection }> {
  const listener = await BroadcastListener.connect(
    "http://127.0.0.1:1/listen",
    options,
  );
  const sse =
    MockSseConnection.instances[MockSseConnection.instances.length - 1];
  return { listener, sse };
}

/** Collect the next `n` runs off a fresh iterator. */
function collectRuns(
  listener: BroadcastListener,
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

/** Let queued microtasks/macrotasks settle. */
async function settle() {
  await new Promise((r) => setTimeout(r, 0));
}

describe("BroadcastListener", () => {
  beforeEach(() => {
    MockSseConnection.instances = [];
    stubFetch();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("sends the signature as the auth header", async () => {
    const { sse } = await connect({ signature: "sha256=abc" });
    expect(sse.headers["X-OBJECTIVEAI-SIGNATURE"]).toBe("sha256=abc");
    expect(sse.headers["Accept"]).toBe("text/event-stream");
  });

  it("omits the auth header when unconfigured", async () => {
    const { sse } = await connect();
    expect(sse.headers["X-OBJECTIVEAI-SIGNATURE"]).toBeUndefined();
  });

  it("rejects connect on a non-2xx response (refused signature)", async () => {
    stubFetch(401);
    await expect(
      BroadcastListener.connect("http://127.0.0.1:1/listen"),
    ).rejects.toThrow(/401/);
  });

  it("yields an envelope per request frame and feeds its stream by id", async () => {
    const { listener, sse } = await connect();
    const runsPromise = collectRuns(listener, 1);
    await settle();

    sse.frame({
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

    const items = (run.response as ResponseItemStream<unknown>).toArray();
    await settle();
    sse.frame({ id: "run-1", value: { n: 1 } });
    sse.frame({ id: "run-1", value: { n: 2 } });
    sse.frame({ id: "run-1", end: true });
    await settle();
    sse.serverClose();
    expect(await items).toEqual([{ n: 1 }, { n: 2 }]);
  });

  it("settles a unary run's promise with its first response frame", async () => {
    const { listener, sse } = await connect();
    const runsPromise = collectRuns(listener, 1);
    await settle();

    sse.frame({ id: "run-u", value: { path_type: "agents/enqueue" } });
    const [run] = await runsPromise;
    expect(run.response).toBeInstanceOf(Promise);

    sse.frame({ id: "run-u", value: { ok: true } });
    sse.frame({ id: "run-u", end: true });
    expect(await run.response).toEqual({ ok: true });
  });

  it("demuxes interleaved runs by id", async () => {
    const { listener, sse } = await connect();
    const runsPromise = collectRuns(listener, 2);
    await settle();

    sse.frame({ id: "a", value: { path_type: "plugins/run" } });
    sse.frame({ id: "b", value: { path_type: "plugins/list" } });
    const [runA, runB] = await runsPromise;
    const itemsA = (runA.response as ResponseItemStream<unknown>).toArray();
    const itemsB = (runB.response as ResponseItemStream<unknown>).toArray();
    await settle();

    sse.frame({ id: "a", value: { from: "a1" } });
    sse.frame({ id: "b", value: { from: "b1" } });
    sse.frame({ id: "a", value: { from: "a2" } });
    sse.frame({ id: "b", end: true });
    sse.frame({ id: "a", end: true });
    await settle();

    expect(await itemsA).toEqual([{ from: "a1" }, { from: "a2" }]);
    expect(await itemsB).toEqual([{ from: "b1" }]);
  });

  it("skips runs whose path_type is unknown, dropping their frames", async () => {
    const { listener, sse } = await connect();
    const runsPromise = collectRuns(listener, 1);
    await settle();

    sse.frame({ id: "mystery", value: { path_type: "not/a/real/path" } });
    sse.frame({ id: "mystery", value: { dropped: true } });
    sse.frame({ id: "mystery", end: true });
    sse.frame({ id: "real", value: { path_type: "plugins/run" } });

    const [run] = await runsPromise;
    expect((run.request as { path_type: string }).path_type).toBe(
      "plugins/run",
    );
  });

  it("delivers live-only: a late subscriber misses earlier runs", async () => {
    const { listener, sse } = await connect();
    const early = collectRuns(listener, 2);
    await settle();

    sse.frame({ id: "first", value: { path_type: "plugins/run" } });
    await settle();

    const late = collectRuns(listener, 1);
    await settle();
    sse.frame({ id: "second", value: { path_type: "plugins/list" } });

    expect(
      (await early).map((r) => (r.request as { path_type: string }).path_type),
    ).toEqual(["plugins/run", "plugins/list"]);
    expect(
      (await late).map((r) => (r.request as { path_type: string }).path_type),
    ).toEqual(["plugins/list"]);
  });

  it("ends every root iterator and open feed when the connection closes", async () => {
    const { listener, sse } = await connect();
    const allRuns = (async () => {
      const out: unknown[] = [];
      for await (const run of listener) out.push(run);
      return out;
    })();
    await settle();

    sse.frame({ id: "s", value: { path_type: "plugins/run" } });
    sse.frame({ id: "u", value: { path_type: "agents/enqueue" } });
    await settle();

    sse.serverClose();
    await settle();

    // Root iterator ends (Rust parity: the stream is over).
    const runs = (await allRuns) as CliCommandListenerExecution[];
    expect(runs).toHaveLength(2);
    // The open stream ends; the unresolved unary settles with the
    // synthesized "run ended" error.
    const stream = runs[0].response as ResponseItemStream<unknown>;
    expect(stream.done).toBe(true);
    expect(await runs[1].response).toMatchObject({
      type: "error",
      message: expect.stringContaining("run ended before any response item"),
    });
  });

  it("close() drops the connection and ends iteration", async () => {
    const { listener, sse } = await connect();
    const allRuns = (async () => {
      const out: unknown[] = [];
      for await (const run of listener) out.push(run);
      return out;
    })();
    await settle();

    listener.close();
    await settle();
    expect(sse.aborted).toBe(true);
    expect(await allRuns).toEqual([]);
  });
});

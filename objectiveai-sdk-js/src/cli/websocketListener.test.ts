// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";

/**
 * Behavior tests for the native `WebSocketListener`: transport (a
 * mocked global WebSocket — nothing connects), the auth preamble,
 * frame routing by id, live-only multi-subscriber delivery, and the
 * Rust-parity close lifecycle.
 */

/** A controllable stand-in for the browser WebSocket. */
class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;
  static instances: MockWebSocket[] = [];

  url: string;
  readyState = MockWebSocket.CONNECTING;
  sent: string[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onclose: ((event: { code: number }) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    if (this.readyState === MockWebSocket.CLOSED) return;
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.({ code: 1000 });
  }

  // ── test drivers ──
  open(): void {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.();
  }

  frame(value: unknown): void {
    this.onmessage?.({ data: JSON.stringify(value) });
  }

  serverClose(code = 1000): void {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.({ code });
  }
}

import { WebSocketListener, ResponseItemStream } from "./websocketListener";
import type { CliCommandListenerExecution } from "./command/listenerExecution";

/** Connect a listener against the newest mock socket. */
async function connect(options?: {
  signature?: string | null;
}): Promise<{ listener: WebSocketListener; ws: MockWebSocket }> {
  const pending = WebSocketListener.connect(
    "ws://127.0.0.1:1/listen",
    options,
  );
  const ws = MockWebSocket.instances[MockWebSocket.instances.length - 1];
  ws.open();
  return { listener: await pending, ws };
}

/** Collect the next `n` runs off a fresh iterator. */
function collectRuns(
  listener: WebSocketListener,
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

describe("WebSocketListener", () => {
  beforeEach(() => {
    MockWebSocket.instances = [];
    vi.stubGlobal("WebSocket", MockWebSocket);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("sends the auth preamble as the connection's first frame", async () => {
    const { ws } = await connect({ signature: "sha256=abc" });
    expect(ws.sent.map((s) => JSON.parse(s))).toEqual([
      { signature: "sha256=abc" },
    ]);
  });

  it("sends a null signature when unconfigured", async () => {
    const { ws } = await connect();
    expect(ws.sent.map((s) => JSON.parse(s))).toEqual([{ signature: null }]);
  });

  it("rejects connect when the socket closes during the handshake", async () => {
    const pending = WebSocketListener.connect("ws://127.0.0.1:1/listen");
    MockWebSocket.instances[0].serverClose(1006);
    await expect(pending).rejects.toThrow(/code 1006/);
  });

  it("yields an envelope per request frame and feeds its stream by id", async () => {
    const { listener, ws } = await connect();
    const runsPromise = collectRuns(listener, 1);

    ws.frame({
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
    ws.frame({ id: "run-1", value: { n: 1 } });
    ws.frame({ id: "run-1", value: { n: 2 } });
    ws.frame({ id: "run-1", end: true });
    expect(await items).toEqual([{ n: 1 }, { n: 2 }]);
  });

  it("settles a unary run's promise with its first response frame", async () => {
    const { listener, ws } = await connect();
    const runsPromise = collectRuns(listener, 1);

    ws.frame({ id: "run-u", value: { path_type: "agents/enqueue" } });
    const [run] = await runsPromise;
    expect(run.response).toBeInstanceOf(Promise);

    ws.frame({ id: "run-u", value: { ok: true } });
    ws.frame({ id: "run-u", end: true });
    expect(await run.response).toEqual({ ok: true });
  });

  it("demuxes interleaved runs by id", async () => {
    const { listener, ws } = await connect();
    const runsPromise = collectRuns(listener, 2);

    ws.frame({ id: "a", value: { path_type: "plugins/run" } });
    ws.frame({ id: "b", value: { path_type: "plugins/list" } });
    const [runA, runB] = await runsPromise;
    const itemsA = (runA.response as ResponseItemStream<unknown>).toArray();
    const itemsB = (runB.response as ResponseItemStream<unknown>).toArray();
    await settle();

    ws.frame({ id: "a", value: { from: "a1" } });
    ws.frame({ id: "b", value: { from: "b1" } });
    ws.frame({ id: "a", value: { from: "a2" } });
    ws.frame({ id: "b", end: true });
    ws.frame({ id: "a", end: true });

    expect(await itemsA).toEqual([{ from: "a1" }, { from: "a2" }]);
    expect(await itemsB).toEqual([{ from: "b1" }]);
  });

  it("skips runs whose path_type is unknown, dropping their frames", async () => {
    const { listener, ws } = await connect();
    const runsPromise = collectRuns(listener, 1);

    ws.frame({ id: "mystery", value: { path_type: "not/a/real/path" } });
    ws.frame({ id: "mystery", value: { dropped: true } });
    ws.frame({ id: "mystery", end: true });
    ws.frame({ id: "real", value: { path_type: "plugins/run" } });

    const [run] = await runsPromise;
    expect((run.request as { path_type: string }).path_type).toBe(
      "plugins/run",
    );
  });

  it("delivers live-only: a late subscriber misses earlier runs", async () => {
    const { listener, ws } = await connect();
    const early = collectRuns(listener, 2);
    await settle();

    ws.frame({ id: "first", value: { path_type: "plugins/run" } });

    const late = collectRuns(listener, 1);
    await settle();
    ws.frame({ id: "second", value: { path_type: "plugins/list" } });

    expect(
      (await early).map((r) => (r.request as { path_type: string }).path_type),
    ).toEqual(["plugins/run", "plugins/list"]);
    expect(
      (await late).map((r) => (r.request as { path_type: string }).path_type),
    ).toEqual(["plugins/list"]);
  });

  it("ends every root iterator and open feed when the connection closes", async () => {
    const { listener, ws } = await connect();
    const allRuns = (async () => {
      const out: unknown[] = [];
      for await (const run of listener) out.push(run);
      return out;
    })();
    await settle();

    ws.frame({ id: "s", value: { path_type: "plugins/run" } });
    ws.frame({ id: "u", value: { path_type: "agents/enqueue" } });
    await settle();

    ws.serverClose();

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
    const { listener, ws } = await connect();
    const allRuns = (async () => {
      const out: unknown[] = [];
      for await (const run of listener) out.push(run);
      return out;
    })();
    await settle();

    listener.close();
    expect(ws.readyState).toBe(MockWebSocket.CLOSED);
    expect(await allRuns).toEqual([]);
  });
});

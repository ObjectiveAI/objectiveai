// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import { WebSocketExecutor } from "./websocket";
import type { CliCommandRequest } from "../request";

/**
 * A controllable stand-in for the browser WebSocket — the tests drive
 * open/message/close by hand. All infra is mocked; nothing connects.
 */
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
  onerror: (() => void) | null = null;
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

  message(value: unknown): void {
    this.onmessage?.({ data: JSON.stringify(value) });
  }

  serverClose(code = 1000): void {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.({ code });
  }
}

const asRequest = (r: unknown): CliCommandRequest => r as CliCommandRequest;

/** Collect all lines of one execute() iteration. */
async function collect(iterable: AsyncIterable<unknown>): Promise<unknown[]> {
  const out: unknown[] = [];
  for await (const line of iterable) out.push(line);
  return out;
}

describe("WebSocketExecutor", () => {
  beforeEach(() => {
    MockWebSocket.instances = [];
    vi.stubGlobal("WebSocket", MockWebSocket);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("connects per execute and sends the auth preamble, then the envelope", () => {
    const executor = new WebSocketExecutor("ws://127.0.0.1:1/execute", {
      signature: "sha256=abc",
      agentArguments: { agent_instance_hierarchy: "Viewer" },
    });
    const request = asRequest({ path_type: "plugins/list" });
    void collect(executor.execute(request));

    expect(MockWebSocket.instances).toHaveLength(1);
    const ws = MockWebSocket.instances[0];
    expect(ws.url).toBe("ws://127.0.0.1:1/execute");
    expect(ws.sent).toHaveLength(0);
    ws.open();
    expect(ws.sent.map((s) => JSON.parse(s))).toEqual([
      { signature: "sha256=abc" },
      {
        agent_arguments: { agent_instance_hierarchy: "Viewer" },
        request: { path_type: "plugins/list" },
      },
    ]);
    ws.serverClose();
  });

  it("sends a null signature and omits agent_arguments when unconfigured", () => {
    const executor = new WebSocketExecutor("ws://127.0.0.1:1/execute");
    void collect(executor.execute(asRequest({ path_type: "plugins/list" })));
    const ws = MockWebSocket.instances[0];
    ws.open();
    expect(ws.sent.map((s) => JSON.parse(s))).toEqual([
      { signature: null },
      { request: { path_type: "plugins/list" } },
    ]);
    ws.serverClose();
  });

  it("yields each JSONL line and ends when the daemon closes", async () => {
    const executor = new WebSocketExecutor("ws://127.0.0.1:1/execute");
    const lines = collect(executor.execute(asRequest({ path_type: "x" })));

    const ws = MockWebSocket.instances[0];
    ws.open();
    ws.message({ hello: "world" });
    ws.message({ type: "error", level: "warn", fatal: null, message: "hm" });
    ws.serverClose();

    expect(await lines).toEqual([
      { hello: "world" },
      { type: "error", level: "warn", fatal: null, message: "hm" },
    ]);
  });

  it("surfaces a refused connection as one in-band error line, then ends", async () => {
    const executor = new WebSocketExecutor("ws://127.0.0.1:1/execute");
    const lines = collect(executor.execute(asRequest({ path_type: "x" })));

    // Closed before open — unreachable daemon or rejected auth.
    MockWebSocket.instances[0].serverClose(1006);

    const collected = await lines;
    expect(collected).toHaveLength(1);
    expect(collected[0]).toMatchObject({ type: "error", level: "error" });
    expect(String((collected[0] as { message: string }).message)).toContain(
      "code 1006",
    );
  });

  it("closes the socket when the consumer breaks early", async () => {
    const executor = new WebSocketExecutor("ws://127.0.0.1:1/execute");
    const iterable = executor.execute(asRequest({ path_type: "x" }));

    const run = (async () => {
      for await (const line of iterable) {
        void line;
        break; // one line is enough
      }
    })();
    // The socket exists once iteration begins (the async body runs
    // synchronously up to its first await).
    const ws = MockWebSocket.instances[0];
    ws.open();
    ws.message({ n: 1 });
    await run;

    expect(ws.readyState).toBe(MockWebSocket.CLOSED);
  });

  it("runs concurrent executes on independent connections", async () => {
    const executor = new WebSocketExecutor("ws://127.0.0.1:1/execute");
    const first = collect(executor.execute(asRequest({ path_type: "a" })));
    const second = collect(executor.execute(asRequest({ path_type: "b" })));

    expect(MockWebSocket.instances).toHaveLength(2);
    const [wsA, wsB] = MockWebSocket.instances;
    wsA.open();
    wsB.open();
    // Interleave.
    wsA.message({ from: "a1" });
    wsB.message({ from: "b1" });
    wsA.message({ from: "a2" });
    wsB.serverClose();
    wsA.serverClose();

    expect(await first).toEqual([{ from: "a1" }, { from: "a2" }]);
    expect(await second).toEqual([{ from: "b1" }]);
  });

  it("wraps an undecodable frame into an in-band error line", async () => {
    const executor = new WebSocketExecutor("ws://127.0.0.1:1/execute");
    const lines = collect(executor.execute(asRequest({ path_type: "x" })));

    const ws = MockWebSocket.instances[0];
    ws.open();
    ws.onmessage?.({ data: "{not json" });
    ws.serverClose();

    const collected = await lines;
    expect(collected).toHaveLength(1);
    expect(collected[0]).toMatchObject({ type: "error" });
  });
});

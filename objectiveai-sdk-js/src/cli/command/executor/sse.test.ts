// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import { SseCommandExecutor } from "./sse";
import type { CliCommandRequest } from "../request";

const encoder = new TextEncoder();

/**
 * A controllable stand-in for one `fetch`ed SSE response — the tests
 * drive events/close by hand. All infra is mocked; nothing connects.
 */
class MockSseConnection {
  static instances: MockSseConnection[] = [];

  url: string;
  method: string;
  headers: Record<string, string>;
  body: string | undefined;
  aborted = false;
  stream: ReadableStream<Uint8Array>;
  #controller!: ReadableStreamDefaultController<Uint8Array>;

  constructor(url: string, init: RequestInit) {
    this.url = url;
    this.method = init.method ?? "GET";
    this.headers = (init.headers ?? {}) as Record<string, string>;
    this.body = init.body as string | undefined;
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
  event(value: unknown): void {
    this.raw(`data: ${JSON.stringify(value)}\n\n`);
  }

  raw(text: string): void {
    this.#controller.enqueue(encoder.encode(text));
  }

  end(): void {
    this.#controller.close();
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

const asRequest = (r: unknown): CliCommandRequest => r as CliCommandRequest;

/** Collect all lines of one execute() iteration. */
async function collect(iterable: AsyncIterable<unknown>): Promise<unknown[]> {
  const out: unknown[] = [];
  for await (const line of iterable) out.push(line);
  return out;
}

/** Let the executor's async connect settle so the mock exists. */
async function settled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

describe("SseCommandExecutor", () => {
  beforeEach(() => {
    MockSseConnection.instances = [];
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("POSTs the envelope with the signature header", async () => {
    stubFetch();
    const executor = new SseCommandExecutor("http://127.0.0.1:1/execute", {
      signature: "sha256=abc",
      agentArguments: { agent_instance_hierarchy: "Viewer" },
    });
    const request = asRequest({ path_type: "plugins/list" });
    const lines = collect(executor.execute(request));
    await settled();

    expect(MockSseConnection.instances).toHaveLength(1);
    const connection = MockSseConnection.instances[0];
    expect(connection.url).toBe("http://127.0.0.1:1/execute");
    expect(connection.method).toBe("POST");
    expect(connection.headers["X-OBJECTIVEAI-SIGNATURE"]).toBe("sha256=abc");
    expect(connection.headers["Accept"]).toBe("text/event-stream");
    expect(JSON.parse(connection.body ?? "")).toEqual({
      agent_arguments: { agent_instance_hierarchy: "Viewer" },
      request: { path_type: "plugins/list" },
    });
    connection.end();
    await lines;
  });

  it("omits the signature header and agent_arguments when unconfigured", async () => {
    stubFetch();
    const executor = new SseCommandExecutor("http://127.0.0.1:1/execute");
    const lines = collect(executor.execute(asRequest({ path_type: "plugins/list" })));
    await settled();

    const connection = MockSseConnection.instances[0];
    expect(connection.headers["X-OBJECTIVEAI-SIGNATURE"]).toBeUndefined();
    expect(JSON.parse(connection.body ?? "")).toEqual({
      request: { path_type: "plugins/list" },
    });
    connection.end();
    await lines;
  });

  it("yields each event line and ends when the body closes", async () => {
    stubFetch();
    const executor = new SseCommandExecutor("http://127.0.0.1:1/execute");
    const lines = collect(executor.execute(asRequest({ path_type: "x" })));
    await settled();

    const connection = MockSseConnection.instances[0];
    connection.event({ hello: "world" });
    connection.event({ type: "error", level: "warn", fatal: null, message: "hm" });
    connection.end();

    expect(await lines).toEqual([
      { hello: "world" },
      { type: "error", level: "warn", fatal: null, message: "hm" },
    ]);
  });

  it("surfaces a refused request (401) as one in-band error line, then ends", async () => {
    stubFetch(401);
    const executor = new SseCommandExecutor("http://127.0.0.1:1/execute");
    const collected = await collect(
      executor.execute(asRequest({ path_type: "x" })),
    );

    expect(collected).toHaveLength(1);
    expect(collected[0]).toMatchObject({ type: "error", level: "error" });
    expect(String((collected[0] as { message: string }).message)).toContain(
      "401",
    );
  });

  it("aborts the request when the consumer breaks early", async () => {
    stubFetch();
    const executor = new SseCommandExecutor("http://127.0.0.1:1/execute");
    const iterable = executor.execute(asRequest({ path_type: "x" }));

    const run = (async () => {
      for await (const line of iterable) {
        void line;
        break; // one line is enough
      }
    })();
    await settled();
    const connection = MockSseConnection.instances[0];
    connection.event({ n: 1 });
    await run;

    expect(connection.aborted).toBe(true);
  });

  it("runs concurrent executes on independent requests", async () => {
    stubFetch();
    const executor = new SseCommandExecutor("http://127.0.0.1:1/execute");
    const first = collect(executor.execute(asRequest({ path_type: "a" })));
    const second = collect(executor.execute(asRequest({ path_type: "b" })));
    await settled();

    expect(MockSseConnection.instances).toHaveLength(2);
    const [a, b] = MockSseConnection.instances;
    // Interleave.
    a.event({ from: "a1" });
    b.event({ from: "b1" });
    a.event({ from: "a2" });
    b.end();
    a.end();

    expect(await first).toEqual([{ from: "a1" }, { from: "a2" }]);
    expect(await second).toEqual([{ from: "b1" }]);
  });

  it("wraps an undecodable event into an in-band error line", async () => {
    stubFetch();
    const executor = new SseCommandExecutor("http://127.0.0.1:1/execute");
    const lines = collect(executor.execute(asRequest({ path_type: "x" })));
    await settled();

    const connection = MockSseConnection.instances[0];
    connection.raw("data: {not json\n\n");
    connection.end();

    const collected = await lines;
    expect(collected).toHaveLength(1);
    expect(collected[0]).toMatchObject({ type: "error" });
  });
});

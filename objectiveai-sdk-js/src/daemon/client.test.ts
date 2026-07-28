// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import { Client } from "./client";
import type { ViewerStreamEvent, ViewerTransport } from "./viewerStream";
import type { CliCommandRequest } from "../cli/command/request";

const encoder = new TextEncoder();

/**
 * Behavior tests for the daemon `Client` in BOTH construction modes:
 * regular (a mocked global `fetch` — nothing connects) and viewer (a
 * fake injected transport recording invokes and scripting channel
 * events). All infra is mocked; no network anywhere.
 */

/** A controllable stand-in for one `fetch`ed SSE response. */
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
        return {
          ok: false,
          status,
          body: null,
          text: async () => "",
          headers: new Headers(),
        };
      }
      const connection = new MockSseConnection(url, init);
      return {
        ok: true,
        status,
        body: connection.stream,
        headers: new Headers(),
      };
    }),
  );
}

/**
 * A fake {@link ViewerTransport}: records every invoke, scripts each
 * command's response, and hands the caller the channel so tests can
 * push stream events by hand.
 */
class FakeTransport implements ViewerTransport {
  invokes: { cmd: string; args: Record<string, unknown> }[] = [];
  channels: { onmessage: (message: never) => void }[] = [];
  respond: (cmd: string, args: Record<string, unknown>) => unknown = () =>
    undefined;

  invoke(cmd: string, args: Record<string, unknown>): Promise<unknown> {
    this.invokes.push({ cmd, args });
    try {
      return Promise.resolve(this.respond(cmd, args));
    } catch (e) {
      return Promise.reject(e);
    }
  }

  channel<T>(): { onmessage: (message: T) => void } {
    const channel = { onmessage: (_message: T) => {} };
    this.channels.push(channel as { onmessage: (message: never) => void });
    return channel;
  }

  /** The stream channel of the most recent streaming invoke. */
  lastChannel<T>(): { onmessage: (message: T) => void } {
    return this.channels[this.channels.length - 1] as {
      onmessage: (message: T) => void;
    };
  }
}

const asRequest = (r: unknown): CliCommandRequest => r as CliCommandRequest;

/** Collect all lines of one execute() iteration. */
async function collect(iterable: AsyncIterable<unknown>): Promise<unknown[]> {
  const out: unknown[] = [];
  for await (const line of iterable) out.push(line);
  return out;
}

/** Let the client's async connect settle so the mock exists. */
async function settled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

describe("Client (regular mode)", () => {
  beforeEach(() => {
    MockSseConnection.instances = [];
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("POSTs the raw request to /execute with signature + identity headers", async () => {
    stubFetch();
    const client = new Client("http://127.0.0.1:1", {
      signature: "sha256=abc",
      identity: { agent_instance_hierarchy: "Viewer", task: false },
    });
    const request = asRequest({ path_type: "plugins/list" });
    const lines = collect(client.execute(request));
    await settled();

    expect(MockSseConnection.instances).toHaveLength(1);
    const connection = MockSseConnection.instances[0];
    expect(connection.url).toBe("http://127.0.0.1:1/execute");
    expect(connection.method).toBe("POST");
    expect(connection.headers["X-OBJECTIVEAI-SIGNATURE"]).toBe("sha256=abc");
    expect(connection.headers["Accept"]).toBe("text/event-stream");
    expect(connection.headers["X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY"]).toBe(
      "Viewer",
    );
    expect(JSON.parse(connection.body ?? "")).toEqual({
      path_type: "plugins/list",
    });
    connection.end();
    await lines;
  });

  it("per-call identity beats the client's own", async () => {
    stubFetch();
    const client = new Client("http://127.0.0.1:1", {
      identity: { agent_id: "client-level", task: false },
    });
    const lines = collect(
      client.execute(asRequest({ path_type: "x" }), {
        identity: { agent_id: "per-call", task: false },
      }),
    );
    await settled();

    const connection = MockSseConnection.instances[0];
    expect(connection.headers["X-OBJECTIVEAI-AGENT-ID"]).toBe("per-call");
    connection.end();
    await lines;
  });

  it("omits signature and identity headers when unconfigured", async () => {
    stubFetch();
    const client = new Client("http://127.0.0.1:1");
    const lines = collect(client.execute(asRequest({ path_type: "x" })));
    await settled();

    const connection = MockSseConnection.instances[0];
    expect(connection.headers["X-OBJECTIVEAI-SIGNATURE"]).toBeUndefined();
    expect(
      connection.headers["X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY"],
    ).toBeUndefined();
    connection.end();
    await lines;
  });

  it("yields each event line and ends when the body closes", async () => {
    stubFetch();
    const client = new Client("http://127.0.0.1:1");
    const lines = collect(client.execute(asRequest({ path_type: "x" })));
    await settled();

    const connection = MockSseConnection.instances[0];
    connection.event({ hello: "world" });
    connection.event({
      type: "error",
      level: "warn",
      fatal: null,
      message: "hm",
    });
    connection.end();

    expect(await lines).toEqual([
      { hello: "world" },
      { type: "error", level: "warn", fatal: null, message: "hm" },
    ]);
  });

  it("surfaces a refused request (401) as one in-band error line, then ends", async () => {
    stubFetch(401);
    const client = new Client("http://127.0.0.1:1");
    const collected = await collect(
      client.execute(asRequest({ path_type: "x" })),
    );

    expect(collected).toHaveLength(1);
    expect(collected[0]).toMatchObject({ type: "error", level: "error" });
    expect(String((collected[0] as { message: string }).message)).toContain(
      "401",
    );
  });

  it("aborts the request when the consumer breaks early", async () => {
    stubFetch();
    const client = new Client("http://127.0.0.1:1");
    const iterable = client.execute(asRequest({ path_type: "x" }));

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

  it("accepts a channel and resolves the owner secret", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string, init: RequestInit) => {
        expect(url).toBe("http://127.0.0.1:1/channels/ch-1/accept");
        expect(init.method).toBe("POST");
        return {
          ok: true,
          status: 200,
          text: async () => JSON.stringify({ secret: "S_owner" }),
        };
      }),
    );
    const client = new Client("http://127.0.0.1:1");
    expect(await client.acceptChannel("ch-1")).toBe("S_owner");
  });

  it("downloads a viewer plugin: sha header + raw bytes", async () => {
    const payload = new Uint8Array([1, 2, 3, 4]);
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        expect(url).toBe(
          "http://127.0.0.1:1/plugins/exampleorg/hello-channel/v0.1.2/viewer",
        );
        return {
          ok: true,
          status: 200,
          headers: new Headers({ "x-objectiveai-sha": "abc123" }),
          body: new ReadableStream<Uint8Array>({
            start(controller) {
              controller.enqueue(payload);
              controller.close();
            },
          }),
        };
      }),
    );
    const client = new Client("http://127.0.0.1:1");
    const plugin = await client.getViewerPlugin(
      "exampleorg",
      "hello-channel",
      "v0.1.2",
    );
    expect(plugin.commitSha).toBe("abc123");
    const chunks: Uint8Array[] = [];
    for await (const chunk of plugin) chunks.push(chunk);
    expect(chunks).toEqual([payload]);
  });
});

describe("Client (viewer mode)", () => {
  it("executes through daemon_execute and yields the streamed lines", async () => {
    const transport = new FakeTransport();
    const client = Client.viewer(transport);
    const lines = collect(client.execute(asRequest({ path_type: "x" })));
    await settled();

    expect(transport.invokes).toHaveLength(1);
    const invoke = transport.invokes[0];
    expect(invoke.cmd).toBe("daemon_execute");
    expect(invoke.args.request).toBe(JSON.stringify({ path_type: "x" }));
    expect(typeof invoke.args.streamId).toBe("string");

    const channel = transport.lastChannel<ViewerStreamEvent>();
    channel.onmessage({ type: "data", data: JSON.stringify({ ok: 1 }) });
    channel.onmessage({ type: "end" });
    expect(await lines).toEqual([{ ok: 1 }]);
  });

  it("mints a channel listener via daemon_channels and folds offers", async () => {
    const transport = new FakeTransport();
    const client = Client.viewer(transport);
    const listener = await client.channelListener();

    expect(transport.invokes[0].cmd).toBe("daemon_channels");
    const channel = transport.lastChannel<ViewerStreamEvent>();
    channel.onmessage({
      type: "data",
      data: JSON.stringify({
        type: "offer",
        offer: { channel_id: "ch-1", key: "demo", task: false },
      }),
    });
    await settled();
    expect(listener.pending().map((offer) => offer.channel_id)).toEqual([
      "ch-1",
    ]);
  });

  it("accepts a channel through daemon_channel_accept", async () => {
    const transport = new FakeTransport();
    transport.respond = (cmd) =>
      cmd === "daemon_channel_accept" ? "S_owner" : undefined;
    const client = Client.viewer(transport);
    expect(await client.acceptChannel("ch-1")).toBe("S_owner");
    expect(transport.invokes[0]).toMatchObject({
      cmd: "daemon_channel_accept",
      args: { channelId: "ch-1" },
    });
  });

  it("downloads a viewer plugin through daemon_viewer_plugin, decoding base64 chunks", async () => {
    const transport = new FakeTransport();
    transport.respond = (cmd) =>
      cmd === "daemon_viewer_plugin" ? { commitSha: "abc123" } : undefined;
    const client = Client.viewer(transport);
    const plugin = await client.getViewerPlugin("o", "n", "v1.0.0");
    expect(plugin.commitSha).toBe("abc123");
    expect(transport.invokes[0].cmd).toBe("daemon_viewer_plugin");

    const channel = transport.lastChannel<
      | { type: "chunk"; data: string }
      | { type: "end" }
      | { type: "error"; message: string }
    >();
    // "AQID" = base64 of [1, 2, 3].
    channel.onmessage({ type: "chunk", data: "AQID" });
    channel.onmessage({ type: "end" });
    const chunks: Uint8Array[] = [];
    for await (const chunk of plugin) chunks.push(chunk);
    expect(chunks).toEqual([new Uint8Array([1, 2, 3])]);
  });

  it("never stamps identity or signature (no such args ride the invoke)", async () => {
    const transport = new FakeTransport();
    const client = Client.viewer(transport);
    const lines = collect(client.execute(asRequest({ path_type: "x" })));
    await settled();

    const args = transport.invokes[0].args;
    expect(Object.keys(args).sort()).toEqual([
      "onEvent",
      "request",
      "streamId",
    ]);
    const channel = transport.lastChannel<ViewerStreamEvent>();
    channel.onmessage({ type: "end" });
    await lines;
  });
});

// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";

/**
 * Bridge tests: plugin iframes post `cli-execute` requests; the host
 * resolves the originating iframe by window identity, runs the
 * request through its JS-native WebSocketExecutor (mocked — nothing
 * connects), and posts every line + the synthetic end marker back
 * into that iframe. Daemon config comes from the mocked Rust
 * `daemon_config` command.
 */

// ── mocks: lib/tauri invoke + the SDK executor ─────────────────────

const harness = vi.hoisted(() => ({
  daemonConfig: {
    address: "ws://127.0.0.1:4242",
    signature: "sha256=abc",
  } as { address: string; signature: string | null } | null,
  /** Constructor args of every WebSocketExecutor the bridge built. */
  constructed: [] as Array<{ url: string; options: unknown }>,
  /** Requests passed to execute(), in order. */
  executed: [] as unknown[],
  /** Script for the next execute() calls: arrays of lines to yield,
   * or an Error to throw. */
  plans: [] as Array<unknown[] | Error>,
  reset(): void {
    harness.daemonConfig = {
      address: "ws://127.0.0.1:4242",
      signature: "sha256=abc",
    };
    harness.constructed.length = 0;
    harness.executed.length = 0;
    harness.plans.length = 0;
  },
}));

vi.mock("./lib/tauri", () => ({
  tauriInvoke: async (cmd: string) => {
    if (cmd !== "daemon_config") throw new Error(`unexpected invoke: ${cmd}`);
    if (!harness.daemonConfig) throw new Error("daemon_config unavailable");
    return harness.daemonConfig;
  },
}));

vi.mock("@objectiveai/sdk", () => ({
  WebSocketExecutor: class {
    constructor(url: string, options: unknown) {
      harness.constructed.push({ url, options });
    }
    execute(request: unknown): AsyncIterable<unknown> {
      harness.executed.push(request);
      const plan = harness.plans.shift() ?? [];
      return (async function* () {
        if (plan instanceof Error) throw plan;
        for (const line of plan) yield line;
      })();
    }
  },
}));

// ── fixtures ────────────────────────────────────────────────────────

type Bridge = typeof import("./plugin-bridge");

/** A registered plugin tab: a real jsdom iframe with a postMessage spy. */
function makeTab(
  bridge: Bridge,
  coords: { owner: string; name: string; version: string },
) {
  const iframe = document.createElement("iframe");
  document.body.appendChild(iframe);
  const spy = vi.spyOn(iframe.contentWindow as Window, "postMessage");
  bridge.registerIframe(
    coords,
    iframe,
    `plugin://localhost/${coords.owner}/${coords.name}/${coords.version}/index.html`,
  );
  return { coords, iframe, spy };
}

const ALPHA = { owner: "objectiveai", name: "alpha", version: "0.0.1" };
const BETA = { owner: "objectiveai", name: "beta", version: "0.0.1" };

/** First-arg payloads of every postMessage a tab received. */
function payloads(tab: { spy: { mock: { calls: unknown[][] } } }): unknown[] {
  return tab.spy.mock.calls.map((call: unknown[]) => call[0]);
}

/** Post a cli-execute message as if from `tab`'s iframe. */
function execFrom(
  tab: { iframe: HTMLIFrameElement },
  id: string,
  request: unknown,
): void {
  window.dispatchEvent(
    new MessageEvent("message", {
      data: { kind: "cli-execute", id, request },
      source: tab.iframe.contentWindow,
    }),
  );
}

/** Let the async run-and-post pipeline settle. */
async function settle() {
  await new Promise((r) => setTimeout(r, 0));
}

describe("plugin-bridge cli-execute", () => {
  let bridge: Bridge;

  beforeEach(async () => {
    // The bridge holds module-level state — re-import a fresh copy.
    vi.resetModules();
    harness.reset();
    document.body.innerHTML = "";
    bridge = await import("./plugin-bridge");
  });

  it("builds the executor from daemon_config and runs the request", async () => {
    const a = makeTab(bridge, ALPHA);
    const request = { path_type: "plugins/list" };
    harness.plans.push([{ n: 1 }]);

    execFrom(a, "invocation-7", request);
    await settle();

    expect(harness.constructed).toEqual([
      {
        url: "ws://127.0.0.1:4242/execute",
        options: {
          signature: "sha256=abc",
          agentArguments: { agent_instance_hierarchy: "Viewer" },
        },
      },
    ]);
    expect(harness.executed).toEqual([request]);
  });

  it("posts every line with the invocation id, then the end marker", async () => {
    const a = makeTab(bridge, ALPHA);
    harness.plans.push([{ n: 1 }, { n: 2 }]);

    execFrom(a, "invocation-1", { path_type: "plugins/list" });
    await settle();

    expect(payloads(a)).toEqual([
      { kind: "plugin-event", type: "cli_command", id: "invocation-1", value: { n: 1 } },
      { kind: "plugin-event", type: "cli_command", id: "invocation-1", value: { n: 2 } },
      { kind: "plugin-event", type: "cli_command", id: "invocation-1", value: { type: "end" } },
    ]);
  });

  it("responses reach the originating iframe only", async () => {
    const a = makeTab(bridge, ALPHA);
    const b = makeTab(bridge, BETA);
    harness.plans.push([{ from: "a" }]);

    execFrom(a, "invocation-1", { path_type: "plugins/list" });
    await settle();

    expect(payloads(a)).toHaveLength(2); // line + end
    expect(b.spy).not.toHaveBeenCalled();
  });

  it("constructs the executor once across invocations", async () => {
    const a = makeTab(bridge, ALPHA);
    harness.plans.push([], []);

    execFrom(a, "invocation-1", { path_type: "plugins/list" });
    execFrom(a, "invocation-2", { path_type: "plugins/list" });
    await settle();

    expect(harness.constructed).toHaveLength(1);
    expect(harness.executed).toHaveLength(2);
  });

  it("surfaces an executor failure as one error line, then the end marker", async () => {
    const a = makeTab(bridge, ALPHA);
    harness.plans.push(new Error("daemon unreachable"));

    execFrom(a, "invocation-1", { path_type: "plugins/list" });
    await settle();

    const posted = payloads(a);
    expect(posted).toHaveLength(2);
    expect(posted[0]).toMatchObject({
      kind: "plugin-event",
      type: "cli_command",
      id: "invocation-1",
      value: { type: "error", message: expect.stringContaining("daemon unreachable") },
    });
    expect(posted[1]).toMatchObject({ value: { type: "end" } });
  });

  it("surfaces a daemon_config failure as error + end, and recovers later", async () => {
    const a = makeTab(bridge, ALPHA);
    harness.daemonConfig = null;

    execFrom(a, "invocation-1", { path_type: "plugins/list" });
    await settle();

    const posted = payloads(a);
    expect(posted).toHaveLength(2);
    expect(posted[0]).toMatchObject({ value: { type: "error" } });
    expect(posted[1]).toMatchObject({ value: { type: "end" } });

    // The failed config fetch doesn't poison later invocations.
    harness.reset();
    harness.plans.push([{ ok: true }]);
    a.spy.mockClear();
    execFrom(a, "invocation-2", { path_type: "plugins/list" });
    await settle();
    expect(payloads(a)).toEqual([
      { kind: "plugin-event", type: "cli_command", id: "invocation-2", value: { ok: true } },
      { kind: "plugin-event", type: "cli_command", id: "invocation-2", value: { type: "end" } },
    ]);
  });

  it("drops cli-execute messages without an invocation id", async () => {
    const a = makeTab(bridge, ALPHA);

    window.dispatchEvent(
      new MessageEvent("message", {
        data: { kind: "cli-execute", request: { path_type: "plugins/list" } },
        source: a.iframe.contentWindow,
      }),
    );
    await settle();

    expect(harness.executed).toEqual([]);
    expect(a.spy).not.toHaveBeenCalled();
  });

  it("drops cli-execute messages from unknown windows", async () => {
    makeTab(bridge, ALPHA);

    window.dispatchEvent(
      new MessageEvent("message", {
        data: {
          kind: "cli-execute",
          id: "invocation-1",
          request: { path_type: "plugins/list" },
        },
        source: null,
      }),
    );
    await settle();

    expect(harness.executed).toEqual([]);
  });

  it("stops delivering after unregister (the run is dropped)", async () => {
    const a = makeTab(bridge, ALPHA);
    harness.plans.push([{ n: 1 }]);

    execFrom(a, "invocation-1", { path_type: "plugins/list" });
    bridge.unregisterIframe(ALPHA);
    // The in-flight run still posts into the (now-unregistered)
    // iframe's window — delivery is by window handle — but NEW
    // executions from it are refused.
    await settle();
    harness.plans.push([{ n: 2 }]);
    a.spy.mockClear();
    execFrom(a, "invocation-2", { path_type: "plugins/list" });
    await settle();

    expect(harness.executed).toHaveLength(1);
    expect(a.spy).not.toHaveBeenCalled();
  });
});

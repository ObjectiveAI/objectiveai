// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";

/**
 * Delivery tests for the plugin bridge.
 *
 * The Rust side emits plugin-destined events on the shared `"plugin"`
 * Tauri channel, each payload carrying the plugin's FULL coordinates
 * in `destination.plugin`. The bridge does no routing beyond delivery:
 * the property under test is that a plugin's iframe receives exactly
 * the events destined to its coordinates — and nothing else — and
 * that the reverse `cli-execute` path stamps the originating iframe's
 * coordinates as the destination (a plugin never claims an identity
 * itself).
 */

// ── lib/tauri mock: a controllable in-memory bus + invoke capture ───

const bus = vi.hoisted(() => {
  const listeners = new Map<
    string,
    Array<(event: { payload: unknown }) => void>
  >();
  const invokes: Array<{ cmd: string; args: unknown }> = [];
  return {
    listeners,
    invokes,
    emit(channel: string, payload: unknown): void {
      for (const handler of listeners.get(channel) ?? []) {
        handler({ payload });
      }
    },
    reset(): void {
      listeners.clear();
      invokes.length = 0;
    },
  };
});

vi.mock("./lib/tauri", () => ({
  tauriListen: async (
    channel: string,
    handler: (event: { payload: unknown }) => void,
  ): Promise<() => void> => {
    const handlers = bus.listeners.get(channel) ?? [];
    handlers.push(handler);
    bus.listeners.set(channel, handlers);
    return () => {
      const current = bus.listeners.get(channel) ?? [];
      const i = current.indexOf(handler);
      if (i !== -1) current.splice(i, 1);
    };
  },
  tauriInvoke: async (cmd: string, args: unknown) => {
    bus.invokes.push({ cmd, args });
    return undefined;
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
/** Same name as ALPHA, different owner — coordinates must be exact. */
const ALPHA_FORK = { owner: "fork", name: "alpha", version: "0.0.1" };

/** A plugin-channel event payload. `cli_command` events carry the
 * caller-minted invocation id. */
function pluginEvent(
  type: "inbound" | "cli_command",
  coords: { owner: string; name: string; version: string },
  value: unknown,
  id = "invocation-1",
) {
  return type === "cli_command"
    ? { type, destination: { plugin: coords }, id, value }
    : { type, destination: { plugin: coords }, value };
}

/** First-arg payloads of every postMessage a tab received. */
function payloads(tab: { spy: { mock: { calls: unknown[][] } } }): unknown[] {
  return tab.spy.mock.calls.map((call: unknown[]) => call[0]);
}

describe("plugin-bridge delivery", () => {
  let bridge: Bridge;

  beforeEach(async () => {
    // The bridge holds module-level state — re-import a fresh copy.
    vi.resetModules();
    bus.reset();
    document.body.innerHTML = "";
    bridge = await import("./plugin-bridge");
    await Promise.resolve();
  });

  it("delivers cli_command events to the destination's iframe only", () => {
    const a = makeTab(bridge, ALPHA);
    const b = makeTab(bridge, BETA);

    bus.emit("plugin", pluginEvent("cli_command", ALPHA, { hello: "world" }));
    bus.emit("plugin", pluginEvent("cli_command", ALPHA, { type: "end" }));

    expect(payloads(a)).toEqual([
      {
        kind: "plugin-event",
        type: "cli_command",
        id: "invocation-1",
        value: { hello: "world" },
      },
      {
        kind: "plugin-event",
        type: "cli_command",
        id: "invocation-1",
        value: { type: "end" },
      },
    ]);
    expect(b.spy).not.toHaveBeenCalled();
  });

  it("delivers inbound events the same way", () => {
    const a = makeTab(bridge, ALPHA);

    bus.emit("plugin", pluginEvent("inbound", ALPHA, { some: "data" }));

    expect(payloads(a)).toEqual([
      { kind: "plugin-event", type: "inbound", value: { some: "data" } },
    ]);
  });

  it("matches on FULL coordinates, not just the name", () => {
    const a = makeTab(bridge, ALPHA);
    const fork = makeTab(bridge, ALPHA_FORK);

    bus.emit("plugin", pluginEvent("cli_command", ALPHA_FORK, { n: 1 }));

    expect(a.spy).not.toHaveBeenCalled();
    expect(payloads(fork)).toEqual([
      {
        kind: "plugin-event",
        type: "cli_command",
        id: "invocation-1",
        value: { n: 1 },
      },
    ]);
  });

  it("drops events for unregistered coordinates", () => {
    const a = makeTab(bridge, ALPHA);

    bus.emit("plugin", pluginEvent("cli_command", BETA, { n: 1 }));
    bus.emit("plugin", {
      type: "cli_command",
      destination: "objectiveai",
      value: { n: 2 },
    });
    bus.emit("plugin", { type: "cli_command", value: { n: 3 } });
    bus.emit("plugin", null);
    bus.emit("plugin", { type: "other", destination: { plugin: ALPHA } });

    expect(a.spy).not.toHaveBeenCalled();
  });

  it("stops delivering after unregister", () => {
    const a = makeTab(bridge, ALPHA);

    bus.emit("plugin", pluginEvent("cli_command", ALPHA, { n: 1 }));
    bridge.unregisterIframe(ALPHA);
    bus.emit("plugin", pluginEvent("cli_command", ALPHA, { n: 2 }));

    expect(payloads(a)).toEqual([
      {
        kind: "plugin-event",
        type: "cli_command",
        id: "invocation-1",
        value: { n: 1 },
      },
    ]);
  });

  it("stamps the originating iframe's coordinates on cli-execute", () => {
    const a = makeTab(bridge, ALPHA);
    makeTab(bridge, BETA);

    const request = { path_type: "plugins/list" };
    window.dispatchEvent(
      new MessageEvent("message", {
        data: { kind: "cli-execute", id: "invocation-7", request },
        source: a.iframe.contentWindow,
      }),
    );

    expect(bus.invokes).toEqual([
      {
        cmd: "cli_execute",
        args: {
          request,
          id: "invocation-7",
          destination: { plugin: ALPHA },
        },
      },
    ]);
  });

  it("drops cli-execute messages without an invocation id", () => {
    const a = makeTab(bridge, ALPHA);

    window.dispatchEvent(
      new MessageEvent("message", {
        data: { kind: "cli-execute", request: { path_type: "plugins/list" } },
        source: a.iframe.contentWindow,
      }),
    );

    expect(bus.invokes).toEqual([]);
  });

  it("drops cli-execute messages from unknown windows", () => {
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
    window.dispatchEvent(
      new MessageEvent("message", {
        data: { kind: "cli-execute" },
        source: null,
      }),
    );

    expect(bus.invokes).toEqual([]);
  });
});

// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";

/**
 * Routing tests for the daemon-stream side of the plugin bridge.
 *
 * The daemon broadcasts EVERY CLI run to the viewer; the bridge is the
 * only filter between that firehose and plugin iframes. The property
 * under test: a plugin's tab receives exactly its own `plugins/run`
 * frames — and nothing else. No other plugins' runs, no non-plugins/run
 * commands, no malformed or unroutable frames.
 */

// ── lib/tauri mock: a controllable in-memory event bus ─────────────

const bus = vi.hoisted(() => {
  const listeners = new Map<
    string,
    Array<(event: { payload: unknown }) => void>
  >();
  return {
    listeners,
    /** Deliver a payload to every listener of a Tauri channel. */
    emit(channel: string, payload: unknown): void {
      for (const handler of listeners.get(channel) ?? []) {
        handler({ payload });
      }
    },
    reset(): void {
      listeners.clear();
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
  tauriInvoke: async () => undefined,
}));

// ── fixtures ────────────────────────────────────────────────────────

type Bridge = typeof import("./plugin-bridge");

/** A registered plugin tab: a real jsdom iframe with a postMessage spy. */
function makeTab(bridge: Bridge, name: string) {
  const iframe = document.createElement("iframe");
  document.body.appendChild(iframe);
  const spy = vi.spyOn(iframe.contentWindow as Window, "postMessage");
  bridge.registerIframe(name, iframe, `plugin://localhost/${name}/index.html`);
  return { name, iframe, spy };
}

/** Wrap a raw daemon frame in the Event::Inbound envelope the Rust
 * side emits on the "objectiveai" channel. */
function daemonEvent(frame: unknown) {
  return {
    type: "inbound",
    destination: "objectiveai",
    sub_type: "daemon",
    value: frame,
  };
}

/** A broadcast request frame: context fields + id + the CLI request. */
function requestFrame(id: string, name: string, pathType = "plugins/run") {
  return {
    agent_instance_hierarchy: "cli",
    id,
    value: { path_type: pathType, owner: "objectiveai", name, version: "0.0.1" },
  };
}

/** A broadcast response frame for the run `id` — a bare `{id, value}`
 * wrapper (responses carry no type tag; the id is the routing). */
function responseFrame(id: string, value: unknown = { hello: "world" }) {
  return { id, value };
}

/** A broadcast terminator frame for the run `id`. */
function endFrame(id: string) {
  return { id, end: true };
}

/** The exact postMessage payload the bridge must deliver for `frame`. */
function delivered(frame: unknown) {
  return {
    kind: "plugin-event",
    type: "inbound",
    sub_type: "plugins_run",
    value: frame,
  };
}

/** First-arg payloads of every postMessage a tab received. */
function payloads(tab: { spy: { mock: { calls: unknown[][] } } }): unknown[] {
  return tab.spy.mock.calls.map((call: unknown[]) => call[0]);
}

describe("plugin-bridge daemon-frame routing", () => {
  let bridge: Bridge;

  beforeEach(async () => {
    // The bridge holds module-level state (iframes, daemonRunOwners,
    // daemonListenerStarted) — re-import a fresh copy per test.
    vi.resetModules();
    bus.reset();
    document.body.innerHTML = "";
    bridge = await import("./plugin-bridge");
    // The mock registers listeners synchronously, but let any pending
    // registration microtasks settle before tests emit.
    await Promise.resolve();
  });

  it("routes a plugins/run request frame to the target tab only", () => {
    const a = makeTab(bridge, "alpha");
    const b = makeTab(bridge, "beta");

    const frame = requestFrame("run-1", "alpha");
    bus.emit("objectiveai", daemonEvent(frame));

    expect(payloads(a)).toEqual([delivered(frame)]);
    expect(b.spy).not.toHaveBeenCalled();
  });

  it("routes response frames to the tab that owns the run id", () => {
    const a = makeTab(bridge, "alpha");
    const b = makeTab(bridge, "beta");

    const req = requestFrame("run-1", "alpha");
    const res1 = responseFrame("run-1", { hello: "world" });
    const res2 = responseFrame("run-1", { done: true });
    bus.emit("objectiveai", daemonEvent(req));
    bus.emit("objectiveai", daemonEvent(res1));
    bus.emit("objectiveai", daemonEvent(res2));

    expect(payloads(a)).toEqual([
      delivered(req),
      delivered(res1),
      delivered(res2),
    ]);
    expect(b.spy).not.toHaveBeenCalled();
  });

  it("keeps two interleaved runs separated per tab", () => {
    const a = makeTab(bridge, "alpha");
    const b = makeTab(bridge, "beta");

    const reqA = requestFrame("run-a", "alpha");
    const reqB = requestFrame("run-b", "beta");
    const resA1 = responseFrame("run-a", { seq: 1 });
    const resB1 = responseFrame("run-b", { seq: 1 });
    const resA2 = responseFrame("run-a", { seq: 2 });
    const resB2 = responseFrame("run-b", { seq: 2 });
    for (const frame of [reqA, reqB, resA1, resB1, resA2, resB2]) {
      bus.emit("objectiveai", daemonEvent(frame));
    }

    expect(payloads(a)).toEqual([
      delivered(reqA),
      delivered(resA1),
      delivered(resA2),
    ]);
    expect(payloads(b)).toEqual([
      delivered(reqB),
      delivered(resB1),
      delivered(resB2),
    ]);
  });

  it("drops runs for plugins without an open tab, including their responses", () => {
    const a = makeTab(bridge, "alpha");

    // "gamma" has no tab: the request must not be delivered anywhere,
    // and — because its id was never remembered — neither may its
    // later response frames.
    bus.emit("objectiveai", daemonEvent(requestFrame("run-g", "gamma")));
    bus.emit("objectiveai", daemonEvent(responseFrame("run-g")));

    expect(a.spy).not.toHaveBeenCalled();
  });

  it("drops non-plugins/run request frames even when value.name matches a tab", () => {
    const a = makeTab(bridge, "alpha");

    bus.emit(
      "objectiveai",
      daemonEvent(requestFrame("run-1", "alpha", "agents/list")),
    );
    // And the id must not have been remembered as alpha's.
    bus.emit("objectiveai", daemonEvent(responseFrame("run-1")));

    expect(a.spy).not.toHaveBeenCalled();
  });

  it("drops response frames with an unknown id", () => {
    const a = makeTab(bridge, "alpha");

    bus.emit("objectiveai", daemonEvent(responseFrame("never-seen")));

    expect(a.spy).not.toHaveBeenCalled();
  });

  it("drops malformed frames without throwing", () => {
    const a = makeTab(bridge, "alpha");

    const malformed: unknown[] = [
      null,
      "not an object",
      42,
      {}, // no id
      { id: 7, value: { path_type: "plugins/run", name: "alpha" } }, // non-string id
      { id: "run-1" }, // request shape but no value
      { id: "run-1", value: "not an object" },
      { id: "run-1", value: { path_type: "plugins/run", name: 5 } }, // non-string name
    ];
    for (const frame of malformed) {
      expect(() => bus.emit("objectiveai", daemonEvent(frame))).not.toThrow();
    }

    expect(a.spy).not.toHaveBeenCalled();
  });

  it("ignores objectiveai-channel events that are not daemon frames", () => {
    const a = makeTab(bridge, "alpha");

    // Same channel, different sub_type / type: never routed to tabs.
    bus.emit("objectiveai", {
      type: "inbound",
      destination: "objectiveai",
      sub_type: "other",
      value: requestFrame("run-1", "alpha"),
    });
    bus.emit("objectiveai", {
      type: "cli_command",
      destination: "objectiveai",
      value: requestFrame("run-2", "alpha"),
    });

    expect(a.spy).not.toHaveBeenCalled();
  });

  it("delivers the terminator to the owner, then evicts the id", () => {
    const a = makeTab(bridge, "alpha");

    const req = requestFrame("run-1", "alpha");
    const res = responseFrame("run-1");
    const end = endFrame("run-1");
    bus.emit("objectiveai", daemonEvent(req));
    bus.emit("objectiveai", daemonEvent(res));
    bus.emit("objectiveai", daemonEvent(end));
    expect(payloads(a)).toEqual([
      delivered(req),
      delivered(res),
      delivered(end),
    ]);

    // The id is gone: anything after the terminator is dropped.
    bus.emit("objectiveai", daemonEvent(responseFrame("run-1")));
    bus.emit("objectiveai", daemonEvent(endFrame("run-1")));
    expect(payloads(a)).toEqual([
      delivered(req),
      delivered(res),
      delivered(end),
    ]);
  });

  it("drops a terminator with an unknown id", () => {
    const a = makeTab(bridge, "alpha");

    bus.emit("objectiveai", daemonEvent(endFrame("never-seen")));

    expect(a.spy).not.toHaveBeenCalled();
  });

  it("stops delivering to a tab after it unregisters", () => {
    const a = makeTab(bridge, "alpha");

    const req = requestFrame("run-1", "alpha");
    bus.emit("objectiveai", daemonEvent(req));
    expect(payloads(a)).toEqual([delivered(req)]);

    bridge.unregisterIframe("alpha");
    bus.emit("objectiveai", daemonEvent(responseFrame("run-1")));

    // Nothing beyond the pre-unregister delivery.
    expect(payloads(a)).toEqual([delivered(req)]);
  });

  it("delivers only plugins_run-shaped payloads under sustained mixed traffic", () => {
    const a = makeTab(bridge, "alpha");
    const b = makeTab(bridge, "beta");

    // A firehose slice: alpha's run, beta's run, an unrelated agents
    // command, an untabbed plugin's run, and stray malformed frames.
    const reqA = requestFrame("run-a", "alpha");
    const resA = responseFrame("run-a");
    bus.emit("objectiveai", daemonEvent(reqA));
    bus.emit("objectiveai", daemonEvent(requestFrame("run-b", "beta")));
    bus.emit(
      "objectiveai",
      daemonEvent({
        agent_instance_hierarchy: "cli",
        id: "run-x",
        value: { path_type: "agents/list" },
      }),
    );
    bus.emit("objectiveai", daemonEvent(requestFrame("run-g", "gamma")));
    bus.emit("objectiveai", daemonEvent({ id: "run-x" }));
    bus.emit("objectiveai", daemonEvent(resA));
    bus.emit("objectiveai", daemonEvent(responseFrame("run-g")));

    // Every payload alpha ever saw is a plugins_run event whose frame
    // id belongs to alpha's runs — nothing more.
    for (const payload of payloads(a) as Array<{
      sub_type: string;
      value: { id: string };
    }>) {
      expect(payload.sub_type).toBe("plugins_run");
      expect(payload.value.id).toBe("run-a");
    }
    expect(payloads(a)).toEqual([delivered(reqA), delivered(resA)]);
    // Beta saw only its own request.
    expect(payloads(b)).toEqual([
      delivered(requestFrame("run-b", "beta")),
    ]);
  });
});

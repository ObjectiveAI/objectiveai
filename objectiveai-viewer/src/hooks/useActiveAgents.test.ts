// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { ActiveAgent } from "./useActiveAgents";

/**
 * Tests for the active-agents tracker: a typed execution-handler
 * registration on the REAL daemon-listener singleton (SDK / tauri /
 * bridge mocked), with per-caller React state. vi.resetModules gives
 * each test fresh module globals; ending the harness feed retires
 * the previous test's pump.
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const harness = vi.hoisted(() => {
  type Feed = { queue: unknown[]; ended: boolean; wake: (() => void) | null };
  const h = {
    feed: null as Feed | null,
    push(execution: unknown) {
      h.feed?.queue.push(execution);
      h.feed?.wake?.();
    },
    endFeed() {
      if (h.feed) {
        h.feed.ended = true;
        h.feed.wake?.();
      }
    },
    newFeed(): Feed {
      const feed: Feed = { queue: [], ended: false, wake: null };
      h.feed = feed;
      return feed;
    },
  };
  return h;
});

vi.mock("../lib/tauri", () => ({
  tauriInvoke: async () => ({ address: "ws://127.0.0.1:1", signature: null }),
}));

vi.mock("../plugin-bridge", () => ({
  deliverInbound: () => {},
}));

vi.mock("@objectiveai/sdk", () => ({
  WebSocketListener: {
    connect: async () => {
      const feed = harness.newFeed();
      return {
        async *[Symbol.asyncIterator]() {
          for (;;) {
            while (feed.queue.length > 0) yield feed.queue.shift();
            if (feed.ended) return;
            await new Promise<void>((r) => {
              feed.wake = r;
            });
            feed.wake = null;
          }
        },
      };
    },
  },
}));

/** A controllable live-only response stream. */
function fakeResponse() {
  const queue: unknown[] = [];
  let done = false;
  let wake: (() => void) | null = null;
  return {
    push(item: unknown) {
      queue.push(item);
      wake?.();
    },
    end() {
      done = true;
      wake?.();
    },
    async *[Symbol.asyncIterator]() {
      for (;;) {
        while (queue.length > 0) yield queue.shift();
        if (done) return;
        await new Promise<void>((r) => {
          wake = r;
        });
        wake = null;
      }
    },
  };
}

function execution(
  pathType: string,
  streaming: boolean | null | undefined,
  response: unknown,
) {
  return {
    request: {
      path_type: pathType,
      ...(streaming === undefined
        ? {}
        : { dangerous_advanced: { stream: streaming } }),
    },
    agentArguments: {},
    response,
  };
}

type UseActiveAgents = () => ActiveAgent[];

function mountProbe(useActiveAgents: UseActiveAgents) {
  let latest: ActiveAgent[] = [];
  function Probe() {
    latest = useActiveAgents();
    return null;
  }
  const container = document.createElement("div");
  const root: Root = createRoot(container);
  act(() => {
    root.render(createElement(Probe));
  });
  return {
    get agents() {
      return latest;
    },
    async settle() {
      await act(async () => {
        await new Promise((r) => setTimeout(r, 0));
      });
    },
    unmount: () =>
      act(() => {
        root.unmount();
      }),
  };
}

describe("useActiveAgents (registered tracker)", () => {
  let useActiveAgents: UseActiveAgents;

  beforeEach(async () => {
    harness.endFeed(); // retire the previous test's pump
    vi.resetModules();
    const listener = await import("../daemon-listener");
    const mod = await import("./useActiveAgents");
    // The viewer-startup order: register handlers, then start.
    mod.registerActiveAgentsHandler();
    listener.startDaemonListener();
    await new Promise((r) => setTimeout(r, 0));
    useActiveAgents = mod.useActiveAgents;
  });

  it("counts an agents/spawn streaming execution's string-Id AIH until its stream ends", async () => {
    const probe = mountProbe(useActiveAgents);
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await probe.settle();

    response.push({ some: "chunk" });
    response.push("Agent/root/1");
    await probe.settle();
    expect(probe.agents).toMatchObject([{ agent_instance_hierarchy: "Agent/root/1" }]);

    response.end();
    await probe.settle();
    expect(probe.agents).toEqual([]);
    probe.unmount();
  });

  it("counts function executions' tagged announcements, not their string execution ids", async () => {
    const probe = mountProbe(useActiveAgents);
    const response = fakeResponse();
    harness.push(execution("functions/execute/standard", true, response));
    await probe.settle();

    response.push("execution-id-123"); // NOT an AIH here
    response.push({
      type: "agent_instance_hierarchy",
      agent_instance_hierarchy: "Agent/a",
    });
    await probe.settle();
    expect(probe.agents).toMatchObject([{ agent_instance_hierarchy: "Agent/a" }]);

    response.end();
    await probe.settle();
    expect(probe.agents).toEqual([]);
    probe.unmount();
  });

  it("ignores non-streaming executions and untracked paths", async () => {
    const probe = mountProbe(useActiveAgents);
    harness.push(execution("agents/spawn", false, Promise.resolve("Agent/x")));
    harness.push(execution("agents/spawn", undefined, Promise.resolve("Agent/y")));
    const other = fakeResponse();
    harness.push(execution("agents/list", true, other));
    other.push("Agent/z");
    await probe.settle();
    expect(probe.agents).toEqual([]);
    probe.unmount();
  });

  it("shares one counter across callers with per-caller state and shared identity", async () => {
    const probeA = mountProbe(useActiveAgents);
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await probeA.settle();
    response.push("Agent/shared");
    await probeA.settle();

    // A second caller mounts LATE and still reads the live snapshot.
    const probeB = mountProbe(useActiveAgents);
    await probeB.settle();
    expect(probeB.agents).toMatchObject([
      { agent_instance_hierarchy: "Agent/shared" },
    ]);
    expect(probeB.agents).toBe(probeA.agents);

    // One caller unmounting doesn't disturb the shared counting.
    probeB.unmount();
    const second = fakeResponse();
    harness.push(execution("functions/execute/swiss_system", true, second));
    await probeA.settle();
    second.push({
      type: "agent_instance_hierarchy",
      agent_instance_hierarchy: "Agent/other",
    });
    await probeA.settle();
    expect(probeA.agents).toMatchObject([
      { agent_instance_hierarchy: "Agent/shared" },
      { agent_instance_hierarchy: "Agent/other" },
    ]);

    response.end();
    second.end();
    await probeA.settle();
    expect(probeA.agents).toEqual([]);
    probeA.unmount();
  });

  it("refcounts a shared AIH across concurrent executions, preserving identity", async () => {
    const probe = mountProbe(useActiveAgents);
    const first = fakeResponse();
    const second = fakeResponse();
    harness.push(execution("agents/spawn", true, first));
    harness.push(execution("functions/execute/standard", true, second));
    await probe.settle();

    first.push("Agent/shared");
    second.push({
      type: "agent_instance_hierarchy",
      agent_instance_hierarchy: "Agent/shared",
    });
    await probe.settle();
    const listed = probe.agents;
    expect(listed).toMatchObject([{ agent_instance_hierarchy: "Agent/shared" }]);

    // Refcount move without membership change: same references.
    first.end();
    await probe.settle();
    expect(probe.agents).toBe(listed);

    second.end();
    await probe.settle();
    expect(probe.agents).toEqual([]);
    probe.unmount();
  });

  it("counts a duplicated announcement within one execution only once", async () => {
    const probe = mountProbe(useActiveAgents);
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await probe.settle();

    response.push("Agent/dup");
    response.push("Agent/dup");
    await probe.settle();
    expect(probe.agents).toMatchObject([{ agent_instance_hierarchy: "Agent/dup" }]);

    response.end();
    await probe.settle();
    expect(probe.agents).toEqual([]);
    probe.unmount();
  });

  it("keeps counting through in-band error items", async () => {
    const probe = mountProbe(useActiveAgents);
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await probe.settle();

    response.push({ type: "error", level: "warn", fatal: null, message: "hm" });
    response.push("Agent/resilient");
    await probe.settle();
    expect(probe.agents).toMatchObject([
      { agent_instance_hierarchy: "Agent/resilient" },
    ]);
    response.end();
    await probe.settle();
    probe.unmount();
  });

  it("refreshes last_active_at (new object) on each announcement", async () => {
    const probe = mountProbe(useActiveAgents);
    const first = fakeResponse();
    harness.push(execution("agents/spawn", true, first));
    await probe.settle();
    first.push("Agent/again");
    await probe.settle();
    const before = probe.agents[0];
    expect(before.last_active_at).toEqual(expect.any(String));

    const second = fakeResponse();
    harness.push(execution("agents/spawn", true, second));
    await probe.settle();
    second.push("Agent/again");
    await probe.settle();

    // Same membership, REPLACED object: every announcement is
    // activity and stamps a fresh last_active_at.
    expect(probe.agents).toHaveLength(1);
    expect(probe.agents[0]).not.toBe(before);
    expect(probe.agents[0].agent_instance_hierarchy).toBe("Agent/again");
    expect(probe.agents[0].last_active_at).toEqual(expect.any(String));
    first.end();
    second.end();
    await probe.settle();
    probe.unmount();
  });

  it("keeps tracking globally while no caller is mounted", async () => {
    const probe = mountProbe(useActiveAgents);
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await probe.settle();
    probe.unmount();

    // Announced with zero subscribers — the global still counts.
    response.push("Agent/quiet");
    await new Promise((r) => setTimeout(r, 0));

    const late = mountProbe(useActiveAgents);
    await late.settle();
    expect(late.agents).toMatchObject([{ agent_instance_hierarchy: "Agent/quiet" }]);
    response.end();
    await late.settle();
    expect(late.agents).toEqual([]);
    late.unmount();
  });
});

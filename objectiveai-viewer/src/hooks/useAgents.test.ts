// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { AgentStatus } from "./useAgents";

/**
 * Tests for useAgents: the one-off `agents/instances/list --all`
 * union'd with the live active-agents tracker (REAL useActiveAgents
 * registration on the real daemon-listener; SDK / tauri / bridge /
 * websocket executor mocked). Live status OVERRIDES the list; ended
 * agents stay, marked inactive.
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
    instanceItems: [] as unknown[],
    instanceRequests: [] as unknown[],
    instancesGate: null as (() => void) | null,
    gateInstances: false,
    reset() {
      h.instanceItems = [];
      h.instanceRequests = [];
      h.instancesGate = null;
      h.gateInstances = false;
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

vi.mock("../lib/websocket-executor", () => ({
  websocketExecutor: async () => ({
    execute(request: unknown) {
      harness.instanceRequests.push(request);
      return (async function* () {
        if (harness.gateInstances) {
          await new Promise<void>((r) => {
            harness.instancesGate = r;
          });
        }
        yield* harness.instanceItems;
      })();
    },
  }),
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
  // Passthrough: the executor's items ARE the typed items.
  agentsInstancesListExecute: (
    executor: { execute: (request: unknown) => AsyncIterable<unknown> },
    request: unknown,
  ) => executor.execute(request),
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

function execution(pathType: string, response: unknown) {
  return {
    request: {
      path_type: pathType,
      dangerous_advanced: { stream: true },
    },
    agentArguments: {},
    response,
  };
}

function instance(hier: string) {
  return {
    agent_instance_hierarchy: hier,
    logged: 3,
    queued: 0,
    tags: [],
  };
}

async function settle() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

type UseAgents = () => AgentStatus[];

function mountProbe(useAgents: UseAgents) {
  let latest: AgentStatus[] = [];
  function Probe() {
    latest = useAgents();
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
    unmount: () =>
      act(() => {
        root.unmount();
      }),
  };
}

describe("useAgents", () => {
  let useAgents: UseAgents;

  beforeEach(async () => {
    harness.endFeed(); // retire the previous test's pump
    harness.reset();
    vi.resetModules();
    const listener = await import("../daemon-listener");
    const activeAgents = await import("./useActiveAgents");
    const mod = await import("./useAgents");
    activeAgents.registerActiveAgentsHandler();
    listener.startDaemonListener();
    await new Promise((r) => setTimeout(r, 0));
    useAgents = mod.useAgents;
  });

  it("lists every instance as inactive, skipping in-band errors", async () => {
    harness.instanceItems = [
      instance("Agent/a"),
      { type: "error", level: "warn", fatal: null, message: "hm" },
      instance("Agent/b"),
    ];
    const probe = mountProbe(useAgents);
    await settle();

    expect(probe.agents).toEqual([
      { agent_instance_hierarchy: "Agent/a", active: false },
      { agent_instance_hierarchy: "Agent/b", active: false },
    ]);
    // One one-off read: everything, no targets.
    expect(harness.instanceRequests).toEqual([{ all: true, targets: [] }]);
    probe.unmount();
  });

  it("the live stream overrides a listed agent's status, then marks it back inactive on end", async () => {
    harness.instanceItems = [instance("Agent/a"), instance("Agent/b")];
    const probe = mountProbe(useAgents);
    await settle();
    const before = probe.agents;
    expect(before.every((agent) => !agent.active)).toBe(true);

    const response = fakeResponse();
    harness.push(execution("agents/spawn", response));
    await settle();
    response.push("Agent/a"); // the AIH announcement
    await settle();

    expect(probe.agents).toEqual([
      { agent_instance_hierarchy: "Agent/a", active: true },
      { agent_instance_hierarchy: "Agent/b", active: false },
    ]);
    // The flipped agent got a NEW object; the untouched one kept its.
    expect(probe.agents[0]).not.toBe(before[0]);
    expect(probe.agents[1]).toBe(before[1]);

    response.end();
    await settle();
    // Ended: marked false, never removed.
    expect(probe.agents).toEqual([
      { agent_instance_hierarchy: "Agent/a", active: false },
      { agent_instance_hierarchy: "Agent/b", active: false },
    ]);
    probe.unmount();
  });

  it("appends stream-discovered agents missing from the list and keeps them after end", async () => {
    harness.instanceItems = [instance("Agent/listed")];
    const probe = mountProbe(useAgents);
    await settle();

    const response = fakeResponse();
    harness.push(execution("functions/execute/standard", response));
    await settle();
    response.push({
      type: "agent_instance_hierarchy",
      agent_instance_hierarchy: "Agent/fresh",
    });
    await settle();

    expect(probe.agents).toEqual([
      { agent_instance_hierarchy: "Agent/listed", active: false },
      { agent_instance_hierarchy: "Agent/fresh", active: true },
    ]);

    response.end();
    await settle();
    expect(probe.agents).toEqual([
      { agent_instance_hierarchy: "Agent/listed", active: false },
      { agent_instance_hierarchy: "Agent/fresh", active: false },
    ]);
    probe.unmount();
  });

  it("shows live agents while the instances read is still in flight, then unions", async () => {
    harness.gateInstances = true;
    harness.instanceItems = [instance("Agent/listed")];
    const probe = mountProbe(useAgents);
    const response = fakeResponse();
    harness.push(execution("agents/spawn", response));
    await settle();
    response.push("Agent/live");
    await settle();

    expect(probe.agents).toEqual([
      { agent_instance_hierarchy: "Agent/live", active: true },
    ]);

    act(() => harness.instancesGate?.());
    await settle();
    // Listed agents come first; the stream-seen one follows.
    expect(probe.agents).toEqual([
      { agent_instance_hierarchy: "Agent/listed", active: false },
      { agent_instance_hierarchy: "Agent/live", active: true },
    ]);
    response.end();
    probe.unmount();
  });

  it("a listed agent that is also live dedupes to one active entry", async () => {
    harness.instanceItems = [instance("Agent/a")];
    const probe = mountProbe(useAgents);
    const response = fakeResponse();
    harness.push(execution("agents/spawn", response));
    await settle();
    response.push("Agent/a");
    await settle();

    expect(probe.agents).toEqual([
      { agent_instance_hierarchy: "Agent/a", active: true },
    ]);
    response.end();
    probe.unmount();
  });

  it("keeps the exact array reference while nothing changes", async () => {
    harness.instanceItems = [instance("Agent/a")];
    const probe = mountProbe(useAgents);
    await settle();
    const first = probe.agents;
    await settle();
    expect(probe.agents).toBe(first);
    probe.unmount();
  });
});

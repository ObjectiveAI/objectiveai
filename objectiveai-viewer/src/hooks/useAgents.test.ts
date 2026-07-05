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

const LISTED_AT = "2026-06-20T00:00:00+00:00";
const LISTED_LAST = "2026-06-21T00:00:00+00:00";

function instance(
  hier: string,
  createdAt: string | null = LISTED_AT,
  lastActiveAt: string | null = LISTED_LAST,
) {
  return {
    agent_instance_hierarchy: hier,
    created_at: createdAt,
    last_active_at: lastActiveAt,
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

/** settle() under vi.useFakeTimers. */
async function settleFake() {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
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
      instance("Agent/b", null, null), // reported, but no logs yet
    ];
    const probe = mountProbe(useAgents);
    await settle();

    expect(probe.agents).toEqual([
      {
        agent_instance_hierarchy: "Agent/a",
        active: false,
        created_at: LISTED_AT,
        last_active_at: LISTED_LAST,
      },
      {
        agent_instance_hierarchy: "Agent/b",
        active: false,
        created_at: null,
        last_active_at: null,
      },
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
      {
        agent_instance_hierarchy: "Agent/a",
        active: true,
        created_at: LISTED_AT,
        // Live activity overrides the CLI's last-active.
        last_active_at: expect.any(String),
      },
      {
        agent_instance_hierarchy: "Agent/b",
        active: false,
        created_at: LISTED_AT,
        last_active_at: LISTED_LAST,
      },
    ]);
    expect(probe.agents[0].last_active_at).not.toBe(LISTED_LAST);
    // The flipped agent got a NEW object; the untouched one kept its.
    expect(probe.agents[0]).not.toBe(before[0]);
    expect(probe.agents[1]).toBe(before[1]);

    response.end();
    await settle();
    // Ended: marked false, never removed. The CLI spawn time held
    // throughout — the list is authoritative for reported agents.
    expect(probe.agents).toEqual([
      {
        agent_instance_hierarchy: "Agent/a",
        active: false,
        created_at: LISTED_AT,
        // The live last-active is RETAINED after the stream ends.
        last_active_at: expect.any(String),
      },
      {
        agent_instance_hierarchy: "Agent/b",
        active: false,
        created_at: LISTED_AT,
        last_active_at: LISTED_LAST,
      },
    ]);
    expect(probe.agents[0].last_active_at).not.toBe(LISTED_LAST);
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
      {
        agent_instance_hierarchy: "Agent/listed",
        active: false,
        created_at: LISTED_AT,
        last_active_at: LISTED_LAST,
      },
      {
        agent_instance_hierarchy: "Agent/fresh",
        active: true,
        created_at: expect.any(String),
        last_active_at: expect.any(String),
      },
    ]);
    const locked = probe.agents[1].created_at;

    response.end();
    await settle();
    // The never-listed agent keeps its locked-in spawn time.
    expect(probe.agents).toEqual([
      {
        agent_instance_hierarchy: "Agent/listed",
        active: false,
        created_at: LISTED_AT,
        last_active_at: LISTED_LAST,
      },
      {
        agent_instance_hierarchy: "Agent/fresh",
        active: false,
        created_at: locked,
        last_active_at: expect.any(String),
      },
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
      {
        agent_instance_hierarchy: "Agent/live",
        active: true,
        created_at: expect.any(String),
        last_active_at: expect.any(String),
      },
    ]);

    act(() => harness.instancesGate?.());
    await settle();
    // Listed agents come first; the stream-seen one follows.
    expect(probe.agents).toEqual([
      {
        agent_instance_hierarchy: "Agent/listed",
        active: false,
        created_at: LISTED_AT,
        last_active_at: LISTED_LAST,
      },
      {
        agent_instance_hierarchy: "Agent/live",
        active: true,
        created_at: expect.any(String),
        last_active_at: expect.any(String),
      },
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

    // The CLI's spawn time wins over the stream lock for a reported
    // agent.
    expect(probe.agents).toEqual([
      {
        agent_instance_hierarchy: "Agent/a",
        active: true,
        created_at: LISTED_AT,
        last_active_at: expect.any(String),
      },
    ]);
    response.end();
    probe.unmount();
  });

  it("locks a stream-only agent's spawn time at its first-seen last-active", async () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(new Date("2026-07-05T10:00:00Z"));
      const probe = mountProbe(useAgents);
      const first = fakeResponse();
      harness.push(execution("agents/spawn", first));
      await settleFake();
      first.push("Agent/solo");
      await settleFake();
      expect(probe.agents).toEqual([
        {
          agent_instance_hierarchy: "Agent/solo",
          active: true,
          created_at: "2026-07-05T10:00:00.000Z",
          last_active_at: "2026-07-05T10:00:00.000Z",
        },
      ]);

      first.end();
      await settleFake();
      // Re-announced an hour later: a HIGHER last-active — the spawn
      // time stays locked at the lowest encountered.
      vi.setSystemTime(new Date("2026-07-05T11:00:00Z"));
      const second = fakeResponse();
      harness.push(execution("agents/spawn", second));
      await settleFake();
      second.push("Agent/solo");
      await settleFake();

      // The spawn lock holds while last-active moved to the newer
      // announcement.
      expect(probe.agents).toEqual([
        {
          agent_instance_hierarchy: "Agent/solo",
          active: true,
          created_at: "2026-07-05T10:00:00.000Z",
          last_active_at: "2026-07-05T11:00:00.000Z",
        },
      ]);
      second.end();
      await settleFake();
      probe.unmount();
    } finally {
      vi.useRealTimers();
    }
  });

  it("the CLI spawn time takes over a stream lock when the list lands later", async () => {
    harness.gateInstances = true;
    harness.instanceItems = [
      instance("Agent/solo", "2026-06-01T00:00:00+00:00"),
    ];
    const probe = mountProbe(useAgents);
    const response = fakeResponse();
    harness.push(execution("agents/spawn", response));
    await settle();
    response.push("Agent/solo");
    await settle();
    // Locked from the stream while the read is in flight...
    expect(probe.agents[0].created_at).toEqual(expect.any(String));
    expect(probe.agents[0].created_at).not.toBe("2026-06-01T00:00:00+00:00");

    act(() => harness.instancesGate?.());
    await settle();
    // ...then the CLI-reported value takes over.
    expect(probe.agents).toEqual([
      {
        agent_instance_hierarchy: "Agent/solo",
        active: true,
        created_at: "2026-06-01T00:00:00+00:00",
        last_active_at: expect.any(String),
      },
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

// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";

/**
 * Tests for useAgentInstanceTags: a DYNAMIC (mount-scoped)
 * registration on the real daemon-listener for `agents/tags/apply`
 * executions — acting only on successful resolutions, matching the
 * response's resolved binding — plus the one-off `agents instances
 * get` populate. SDK / tauri / bridge / websocket executor mocked.
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
  agentsInstancesGetExecute: (
    executor: { execute: (request: unknown) => AsyncIterable<unknown> },
    request: unknown,
  ) => executor.execute(request),
}));

/** One agents/tags/apply broadcast execution with a test-held
 * response promise. */
function applyExecution(name: string) {
  let resolve!: (value: unknown) => void;
  let reject!: (reason?: unknown) => void;
  const response = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  // The pump ignores rejections we never observe in-test.
  response.catch(() => {});
  return {
    execution: {
      request: {
        path_type: "agents/tags/apply",
        name,
        target: { by: "agent_instance", agent_instance: "x" },
      },
      agentArguments: {},
      response,
    },
    /** Resolve as a successful binding to `hierarchy`. */
    bindTo(hierarchy: string) {
      resolve({
        by: "agent_instance",
        name,
        agent_instance: hierarchy.split("/").pop(),
        parent_agent_instance_hierarchy: hierarchy
          .split("/")
          .slice(0, -1)
          .join("/"),
        agent_instance_hierarchy: hierarchy,
      });
    },
    /** Resolve as a grouped (non-instance) binding. */
    bindGrouped() {
      resolve({
        by: "agent",
        name,
        parent_agent_instance_hierarchy: "cli",
        tag_group_id: 7,
        agent_spec: {},
      });
    },
    /** Resolve with an in-band CliError. */
    fail() {
      resolve({ type: "error", level: "error", fatal: null, message: "no" });
    },
    /** Reject outright (socket closed mid-execution). */
    die() {
      reject(new Error("run ended"));
    },
  };
}

function instanceItem(hier: string, tags: string[]) {
  return {
    agent_instance_hierarchy: hier,
    created_at: null,
    last_active_at: null,
    logged: 0,
    queued: 0,
    tags,
  };
}

async function settle() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

type UseTags = (hierarchy: string) => string[];

function mountProbe(useTags: UseTags, hierarchy: string) {
  let latest: string[] = [];
  function Probe() {
    latest = useTags(hierarchy);
    return null;
  }
  const container = document.createElement("div");
  const root: Root = createRoot(container);
  act(() => {
    root.render(createElement(Probe));
  });
  return {
    get tags() {
      return latest;
    },
    unmount: () =>
      act(() => {
        root.unmount();
      }),
  };
}

const AIH = "cli/me";

describe("useAgentInstanceTags", () => {
  let useAgentInstanceTags: UseTags;

  beforeEach(async () => {
    harness.endFeed(); // retire the previous test's pump
    harness.reset();
    vi.resetModules();
    const listener = await import("../daemon-listener");
    const mod = await import("./useAgentInstanceTags");
    listener.startDaemonListener();
    await new Promise((r) => setTimeout(r, 0));
    useAgentInstanceTags = mod.useAgentInstanceTags;
  });

  it("populates from agents instances get, only from the matching item", async () => {
    harness.instanceItems = [
      { type: "error", level: "warn", fatal: null, message: "hm" },
      instanceItem("cli/other", ["stray"]),
      instanceItem(AIH, ["alpha", "beta"]),
    ];
    const probe = mountProbe(useAgentInstanceTags, AIH);
    await settle();

    expect(probe.tags).toEqual(["alpha", "beta"]);
    expect(harness.instanceRequests).toEqual([
      {
        targets: [
          {
            by: "direct",
            agent_instance: "me",
            parent_agent_instance_hierarchy: "cli",
          },
        ],
      },
    ]);
    probe.unmount();
  });

  it("adds a tag bound to this AIH only once its promise resolves ok", async () => {
    const probe = mountProbe(useAgentInstanceTags, AIH);
    await settle();

    const apply = applyExecution("fresh");
    harness.push(apply.execution);
    await settle();
    // Announced but unresolved: nothing yet.
    expect(probe.tags).toEqual([]);

    await act(async () => {
      apply.bindTo(AIH);
    });
    await settle();
    expect(probe.tags).toEqual(["fresh"]);
    probe.unmount();
  });

  it("ignores error resolutions and rejections", async () => {
    const probe = mountProbe(useAgentInstanceTags, AIH);
    await settle();

    const errored = applyExecution("nope");
    const dead = applyExecution("gone");
    harness.push(errored.execution);
    harness.push(dead.execution);
    await settle();
    await act(async () => {
      errored.fail();
      dead.die();
    });
    await settle();

    expect(probe.tags).toEqual([]);
    probe.unmount();
  });

  it("removes a held tag when it successfully binds to a different AIH", async () => {
    harness.instanceItems = [instanceItem(AIH, ["mine", "keep"])];
    const probe = mountProbe(useAgentInstanceTags, AIH);
    await settle();
    expect(probe.tags).toEqual(["mine", "keep"]);

    // A failed steal changes nothing.
    const failed = applyExecution("mine");
    harness.push(failed.execution);
    await settle();
    await act(async () => {
      failed.fail();
    });
    await settle();
    expect(probe.tags).toEqual(["mine", "keep"]);

    // A successful re-bind elsewhere removes it.
    const stolen = applyExecution("mine");
    harness.push(stolen.execution);
    await settle();
    await act(async () => {
      stolen.bindTo("cli/thief");
    });
    await settle();
    expect(probe.tags).toEqual(["keep"]);
    probe.unmount();
  });

  it("no-ops (same reference) for unheld tags bound elsewhere", async () => {
    harness.instanceItems = [instanceItem(AIH, ["held"])];
    const probe = mountProbe(useAgentInstanceTags, AIH);
    await settle();
    const before = probe.tags;

    const unrelated = applyExecution("someone-elses");
    harness.push(unrelated.execution);
    await settle();
    await act(async () => {
      unrelated.bindTo("cli/other");
    });
    await settle();

    expect(probe.tags).toBe(before);
    probe.unmount();
  });

  it("removes a held tag on a successful grouped (non-instance) re-bind", async () => {
    harness.instanceItems = [instanceItem(AIH, ["mine"])];
    const probe = mountProbe(useAgentInstanceTags, AIH);
    await settle();
    expect(probe.tags).toEqual(["mine"]);

    const grouped = applyExecution("mine");
    harness.push(grouped.execution);
    await settle();
    await act(async () => {
      grouped.bindGrouped();
    });
    await settle();

    expect(probe.tags).toEqual([]);
    probe.unmount();
  });

  it("runs against the empty list before the populate lands, then populates", async () => {
    harness.gateInstances = true;
    harness.instanceItems = [instanceItem(AIH, ["alpha", "early"])];
    const probe = mountProbe(useAgentInstanceTags, AIH);

    // The registration is live while the read is still gated.
    const early = applyExecution("early");
    harness.push(early.execution);
    await settle();
    await act(async () => {
      early.bindTo(AIH);
    });
    await settle();
    expect(probe.tags).toEqual(["early"]);

    act(() => harness.instancesGate?.());
    await settle();
    // The populate replaces state (the read reflects the apply).
    expect(probe.tags).toEqual(["alpha", "early"]);
    probe.unmount();
  });

  it("unmount unregisters — a late resolution changes nothing", async () => {
    const probe = mountProbe(useAgentInstanceTags, AIH);
    await settle();
    const apply = applyExecution("late");
    harness.push(apply.execution);
    await settle();

    probe.unmount();
    await act(async () => {
      apply.bindTo(AIH);
    });
    await settle();
    // No act warnings, no crash — and a remount starts clean.
    const again = mountProbe(useAgentInstanceTags, AIH);
    await settle();
    expect(again.tags).toEqual([]);
    again.unmount();
  });
});

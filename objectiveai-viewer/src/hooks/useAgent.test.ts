// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { UseAgentResult } from "./useAgent";

/**
 * Tests for useAgent: the one-off `agents/logs/list` read (cut where
 * live coverage begins) plus the live per-response merge fold over
 * the REAL agentCompletions store, driven through the real
 * daemon-listener singleton (SDK / tauri / bridge / websocket
 * executor mocked). The SDK merge functions are marker combiners so
 * the fold's call pattern — exactly one merge per continuation
 * chunk, never a re-merge — is observable.
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const harness = vi.hoisted(() => {
  type Feed = { queue: unknown[]; ended: boolean; wake: (() => void) | null };
  const h = {
    // ── daemon-listener feed (drives the store) ──
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
    // ── logs read (the one-off fetch) ──
    logItems: [] as unknown[],
    logRequests: [] as unknown[],
    logsGate: null as (() => void) | null,
    gateLogs: false,
    // ── merge instrumentation ──
    agentMergeCalls: 0,
    vectorMergeCalls: 0,
    reset() {
      h.logItems = [];
      h.logRequests = [];
      h.logsGate = null;
      h.gateLogs = false;
      h.agentMergeCalls = 0;
      h.vectorMergeCalls = 0;
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
      harness.logRequests.push(request);
      return (async function* () {
        if (harness.gateLogs) {
          await new Promise<void>((r) => {
            harness.logsGate = r;
          });
        }
        yield* harness.logItems;
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
  agentsLogsListExecute: (
    executor: { execute: (request: unknown) => AsyncIterable<unknown> },
    request: unknown,
  ) => executor.execute(request),
  // Marker combiners: keep `a`'s identity fields, count merges.
  agentCompletionsResponseStreamingAgentCompletionChunkMerged: (
    a: Record<string, unknown>,
    b: Record<string, unknown>,
  ) => {
    harness.agentMergeCalls += 1;
    return [{ ...a, ...b, id: a.id, via: "agent" }, true];
  },
  vectorCompletionsResponseStreamingAgentCompletionChunkMerged: (
    a: Record<string, unknown>,
    b: Record<string, unknown>,
  ) => {
    harness.vectorMergeCalls += 1;
    return [{ ...a, ...b, id: a.id, via: "vector" }, true];
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

function chunk(hier: string, id: string, content: string) {
  return {
    agent_instance_hierarchy: hier,
    id,
    messages: [{ content }],
    object: "agent.completion.chunk",
  };
}

/** A log row: only `type` + `response_id` matter to the cut. */
function logRow(type: string, responseId: string, id: number) {
  return {
    type,
    response_id: responseId,
    id,
    agent_instance_hierarchy: "Agent/a",
    delivered_at: "2026-01-01T00:00:00Z",
    sender_agent_instance_hierarchy: "Sender",
  };
}

async function settle() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

type UseAgent = (hierarchy: string) => UseAgentResult;

function mountProbe(useAgent: UseAgent, hierarchy: string) {
  let latest: UseAgentResult = { logs: null, completions: [] };
  function Probe() {
    latest = useAgent(hierarchy);
    return null;
  }
  const container = document.createElement("div");
  const root: Root = createRoot(container);
  act(() => {
    root.render(createElement(Probe));
  });
  return {
    get result() {
      return latest;
    },
    unmount: () =>
      act(() => {
        root.unmount();
      }),
  };
}

describe("useAgent", () => {
  let useAgent: UseAgent;

  beforeEach(async () => {
    harness.endFeed(); // retire the previous test's pump
    harness.reset();
    vi.resetModules();
    const listener = await import("../daemon-listener");
    const store = await import("../listener-handlers/agentCompletions");
    const mod = await import("./useAgent");
    store.registerAgentCompletionsHandler();
    listener.startDaemonListener();
    await new Promise((r) => setTimeout(r, 0));
    useAgent = mod.useAgent;
  });

  it("logs are null in flight, then the full history with no live entry", async () => {
    harness.gateLogs = true;
    harness.logItems = [
      logRow("agent_completion_request", "cmpl-old", 1),
      logRow("assistant_response", "cmpl-old", 2),
    ];
    const probe = mountProbe(useAgent, "Agent/a");
    await settle();
    expect(probe.result.logs).toBeNull();

    act(() => harness.logsGate?.());
    await settle();
    expect(probe.result.logs).toHaveLength(2);
    expect(probe.result.completions).toEqual([]);
    // One one-off read, targeted at the split AIH.
    expect(harness.logRequests).toHaveLength(1);
    expect(harness.logRequests[0]).toMatchObject({
      pending: false,
      targets: [
        {
          by: "direct",
          agent_instance: "a",
          parent_agent_instance_hierarchy: "Agent",
        },
      ],
    });
    probe.unmount();
  });

  it("cuts the history at the live segment's completion-request row", async () => {
    harness.logItems = [
      logRow("agent_completion_request", "cmpl-old", 1),
      logRow("assistant_response", "cmpl-old", 2),
      logRow("agent_completion_request", "cmpl-live", 3),
      logRow("assistant_response", "cmpl-live", 4),
    ];
    const probe = mountProbe(useAgent, "Agent/a");
    const response = fakeResponse();
    harness.push(execution("agents/spawn", response));
    await settle();
    response.push(chunk("Agent/a", "cmpl-live", "live"));
    await settle();

    // Everything from the live request row on is covered by the live
    // chunks; only the older history remains.
    expect(probe.result.logs?.map((l) => l.response_id)).toEqual([
      "cmpl-old",
      "cmpl-old",
    ]);
    expect(probe.result.completions).toHaveLength(1);
    response.end();
    probe.unmount();
  });

  it("cuts at the function execution's request row via the stored root id", async () => {
    harness.logItems = [
      logRow("assistant_response", "cmpl-old", 1),
      logRow("function_execution_request", "exec-1", 2),
      logRow("vector_completion_request", "vc-1", 3),
      logRow("assistant_response", "cmpl-v", 4),
    ];
    const probe = mountProbe(useAgent, "Agent/a");
    const response = fakeResponse();
    harness.push(execution("functions/execute/standard", response));
    await settle();
    response.push({
      object: "scalar.function.execution.chunk",
      id: "exec-1",
      tasks: [
        {
          object: "vector.completion.chunk",
          id: "vc-1",
          completions: [chunk("Agent/a", "cmpl-v", "v")],
        },
      ],
    });
    await settle();

    // "exec-1" is no chunk's id — the cut comes from the segment's
    // functionExecutionResponseId, and it covers the whole execution.
    expect(probe.result.logs?.map((l) => l.response_id)).toEqual(["cmpl-old"]);
    response.end();
    probe.unmount();
  });

  it("merges same-response-id runs into one aggregate each, in order", async () => {
    const probe = mountProbe(useAgent, "Agent/a");
    const response = fakeResponse();
    harness.push(execution("agents/spawn", response));
    await settle();

    response.push(chunk("Agent/a", "cmpl-1", "a"));
    response.push(chunk("Agent/a", "cmpl-1", "b"));
    response.push(chunk("Agent/a", "cmpl-2", "c"));
    await settle();

    const completions = probe.result.completions;
    expect(completions).toHaveLength(2);
    expect(completions.map((c) => c.id)).toEqual(["cmpl-1", "cmpl-2"]);
    // The first aggregate merged once (a+b); the second is the raw
    // solo chunk.
    expect(harness.agentMergeCalls).toBe(1);
    response.end();
    probe.unmount();
  });

  it("consumes each raw chunk exactly once across notifications", async () => {
    const probe = mountProbe(useAgent, "Agent/a");
    const response = fakeResponse();
    harness.push(execution("agents/spawn", response));
    await settle();

    response.push(chunk("Agent/a", "cmpl-1", "a"));
    await settle();
    response.push(chunk("Agent/a", "cmpl-1", "b"));
    await settle();
    response.push(chunk("Agent/a", "cmpl-1", "c"));
    await settle();

    // 3 chunks = 2 continuations = exactly 2 merges. A re-fold of
    // already-consumed chunks would have made 3 (1+2).
    expect(harness.agentMergeCalls).toBe(2);
    expect(probe.result.completions).toHaveLength(1);
    response.end();
    probe.unmount();
  });

  it("a new stream's generation restarts the fold", async () => {
    const probe = mountProbe(useAgent, "Agent/a");
    const old = fakeResponse();
    harness.push(execution("agents/spawn", old));
    await settle();
    old.push(chunk("Agent/a", "cmpl-1", "old-a"));
    old.push(chunk("Agent/a", "cmpl-1", "old-b"));
    old.end();
    await settle();
    expect(probe.result.completions).toHaveLength(1);

    const fresh = fakeResponse();
    harness.push(execution("agents/spawn", fresh));
    await settle();
    const latest = chunk("Agent/a", "cmpl-1", "new");
    fresh.push(latest);
    await settle();

    // The old aggregates are gone — only the new stream's solo chunk.
    expect(probe.result.completions).toEqual([latest]);
    fresh.end();
    probe.unmount();
  });

  it("vector-shaped chunks (with index) merge via the vector function", async () => {
    const probe = mountProbe(useAgent, "Agent/a");
    const response = fakeResponse();
    harness.push(execution("functions/execute/standard", response));
    await settle();

    const vectorChunk = (content: string) => ({
      ...chunk("Agent/a", "cmpl-v", content),
      index: 0,
    });
    const push = (c: unknown) =>
      response.push({
        object: "scalar.function.execution.chunk",
        id: "exec-1",
        tasks: [
          { object: "vector.completion.chunk", id: "vc-1", completions: [c] },
        ],
      });
    push(vectorChunk("a"));
    push(vectorChunk("b"));
    await settle();

    expect(harness.vectorMergeCalls).toBe(1);
    expect(harness.agentMergeCalls).toBe(0);
    expect(probe.result.completions).toHaveLength(1);
    response.end();
    probe.unmount();
  });

  it("re-renders and re-cuts as the live segment grows after the read", async () => {
    harness.logItems = [
      logRow("assistant_response", "cmpl-old", 1),
      logRow("agent_completion_request", "cmpl-late", 2),
    ];
    const probe = mountProbe(useAgent, "Agent/a");
    await settle();
    // Read resolved before any live activity: nothing is covered.
    expect(probe.result.logs).toHaveLength(2);

    const response = fakeResponse();
    harness.push(execution("agents/spawn", response));
    await settle();
    response.push(chunk("Agent/a", "cmpl-late", "now live"));
    await settle();

    // The stream arriving later re-cuts the already-loaded history.
    expect(probe.result.logs?.map((l) => l.response_id)).toEqual(["cmpl-old"]);
    expect(probe.result.completions).toHaveLength(1);
    response.end();
    probe.unmount();
  });
});

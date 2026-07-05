// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";

/**
 * Tests for the per-agent completion store: registrations on the
 * REAL daemon-listener singleton (SDK / tauri / bridge mocked) that
 * keep each agent's most recent completion segment as a raw,
 * unmerged chunk list — fed by spawn chunks directly and by the
 * recursive walk over function-execution chunks, with the response
 * `id` as the reset rule. vi.resetModules gives each test fresh
 * module globals; ending the harness feed retires the previous
 * test's pump.
 */

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

/** An agent-completion chunk (spawn / vector / reasoning shape). */
function chunk(hier: string, id: string, content: string) {
  return {
    agent_instance_hierarchy: hier,
    id,
    messages: [{ content }],
    object: "agent.completion.chunk",
  };
}

/** A function-execution chunk with a vector completion task, a nested
 * function task holding another vector task, and a root reasoning
 * summary — the walk must find completions at every level. */
function functionChunk(opts: {
  vector?: ReturnType<typeof chunk>[];
  nested?: ReturnType<typeof chunk>[];
  reasoning?: ReturnType<typeof chunk>;
}) {
  return {
    object: "scalar.function.execution.chunk",
    id: "exec-1",
    ...(opts.reasoning !== undefined ? { reasoning: opts.reasoning } : {}),
    tasks: [
      {
        object: "vector.completion.chunk",
        id: "vc-1",
        completions: opts.vector ?? [],
      },
      {
        object: "vector.function.execution.chunk",
        id: "fn-1",
        tasks: [
          {
            object: "vector.completion.chunk",
            id: "vc-2",
            completions: opts.nested ?? [],
          },
        ],
      },
    ],
  };
}

async function settle() {
  await new Promise((r) => setTimeout(r, 0));
}

describe("agentCompletions (registered store)", () => {
  let mod: typeof import("./agentCompletions");

  beforeEach(async () => {
    harness.endFeed(); // retire the previous test's pump
    vi.resetModules();
    const listener = await import("../daemon-listener");
    mod = await import("./agentCompletions");
    // The viewer-startup order: register handlers, then start.
    mod.registerAgentCompletionsHandler();
    listener.startDaemonListener();
    await settle();
  });

  it("stores a streaming spawn's chunks per AIH, unmerged, in order", async () => {
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await settle();

    const first = chunk("Agent/a", "cmpl-1", "hel");
    const second = chunk("Agent/a", "cmpl-1", "lo");
    response.push(first);
    response.push(second);
    await settle();

    // Raw chunks, exact objects, arrival order — no merging.
    expect(mod.agentCompletion("Agent/a")).toEqual([first, second]);
    expect(mod.agentCompletion("Agent/a")?.[0]).toBe(first);
    response.end();
  });

  it("ignores bare-string Id items and in-band error items", async () => {
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await settle();

    response.push("Agent/a"); // the AIH announcement — not a chunk
    response.push({ type: "error", level: "warn", fatal: null, message: "hm" });
    response.push(chunk("Agent/a", "cmpl-1", "hi"));
    await settle();

    expect(mod.agentCompletion("Agent/a")).toHaveLength(1);
    expect(mod.agentCompletions().size).toBe(1);
    response.end();
  });

  it("ignores non-streaming executions", async () => {
    harness.push(execution("agents/spawn", false, Promise.resolve("Agent/x")));
    harness.push(
      execution("agents/spawn", undefined, Promise.resolve("Agent/y")),
    );
    harness.push(
      execution(
        "functions/execute/standard",
        false,
        Promise.resolve("exec-id"),
      ),
    );
    await settle();
    expect(mod.agentCompletions().size).toBe(0);
  });

  it("keeps the segment after the stream ends", async () => {
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await settle();
    response.push(chunk("Agent/a", "cmpl-1", "kept"));
    response.end();
    await settle();

    expect(mod.agentCompletion("Agent/a")).toHaveLength(1);
  });

  it("resets the entry to the solo chunk when the response id changes", async () => {
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await settle();

    response.push(chunk("Agent/a", "cmpl-1", "old-1"));
    response.push(chunk("Agent/a", "cmpl-1", "old-2"));
    await settle();
    expect(mod.agentCompletion("Agent/a")).toHaveLength(2);

    // New response id — even mid-stream — starts the segment over.
    const fresh = chunk("Agent/a", "cmpl-2", "new-1");
    response.push(fresh);
    await settle();
    expect(mod.agentCompletion("Agent/a")).toEqual([fresh]);

    // And the same id accumulates again.
    response.push(chunk("Agent/a", "cmpl-2", "new-2"));
    await settle();
    expect(mod.agentCompletion("Agent/a")).toHaveLength(2);
    response.end();
  });

  it("a newer execution's chunks supersede via their fresh response id", async () => {
    const old = fakeResponse();
    harness.push(execution("agents/spawn", true, old));
    await settle();
    old.push(chunk("Agent/a", "cmpl-1", "old-1"));
    old.push(chunk("Agent/a", "cmpl-1", "old-2"));
    old.end();
    await settle();

    const fresh = fakeResponse();
    harness.push(execution("agents/spawn", true, fresh));
    await settle();
    const latest = chunk("Agent/a", "cmpl-2", "new-1");
    fresh.push(latest);
    await settle();

    expect(mod.agentCompletion("Agent/a")).toEqual([latest]);
    fresh.end();
  });

  it("walks function-execution chunks for nested agent completions", async () => {
    const response = fakeResponse();
    harness.push(execution("functions/execute/standard", true, response));
    await settle();

    response.push("execution-id-123"); // the Id item — ignored
    response.push({
      type: "agent_instance_hierarchy",
      agent_instance_hierarchy: "Agent/announced",
    }); // the tagged announcement — ignored
    const top = chunk("Agent/vec", "cmpl-v", "top");
    const deep = chunk("Agent/deep", "cmpl-d", "deep");
    const summary = chunk("Agent/reason", "cmpl-r", "why");
    response.push(
      functionChunk({ vector: [top], nested: [deep], reasoning: summary }),
    );
    await settle();

    // Completions surface from every nesting level; the announced-only
    // agent has no chunks.
    expect(mod.agentCompletion("Agent/vec")).toEqual([top]);
    expect(mod.agentCompletion("Agent/deep")).toEqual([deep]);
    expect(mod.agentCompletion("Agent/reason")).toEqual([summary]);
    expect(mod.agentCompletion("Agent/announced")).toBeUndefined();
    expect(mod.agentCompletions().size).toBe(3);
    response.end();
  });

  it("accumulates one agent's chunks across successive function-execution chunks", async () => {
    const response = fakeResponse();
    harness.push(execution("functions/execute/swiss_system", true, response));
    await settle();

    const first = chunk("Agent/vec", "cmpl-v", "a");
    const second = chunk("Agent/vec", "cmpl-v", "b");
    response.push(functionChunk({ vector: [first] }));
    response.push(functionChunk({ vector: [second] }));
    await settle();

    expect(mod.agentCompletion("Agent/vec")).toEqual([first, second]);
    response.end();
  });

  it("tracks concurrent executions' distinct AIHs independently", async () => {
    const spawn = fakeResponse();
    const fn = fakeResponse();
    harness.push(execution("agents/spawn", true, spawn));
    harness.push(execution("functions/execute/standard", true, fn));
    await settle();

    spawn.push(chunk("Agent/a", "cmpl-a", "a1"));
    fn.push(functionChunk({ vector: [chunk("Agent/b", "cmpl-b", "b1")] }));
    spawn.push(chunk("Agent/a", "cmpl-a", "a2"));
    await settle();

    expect(mod.agentCompletion("Agent/a")).toHaveLength(2);
    expect(mod.agentCompletion("Agent/b")).toHaveLength(1);
    spawn.end();
    fn.end();
  });
});

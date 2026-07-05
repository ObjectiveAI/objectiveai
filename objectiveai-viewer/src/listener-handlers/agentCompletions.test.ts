// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";

/**
 * Tests for the per-agent completion store: a registration on the
 * REAL daemon-listener singleton (SDK / tauri / bridge mocked) that
 * keeps each agent's most recent streaming-spawn segment as a raw,
 * unmerged chunk list. vi.resetModules gives each test fresh module
 * globals; ending the harness feed retires the previous test's pump.
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

function chunk(hier: string, content: string) {
  return {
    agent_instance_hierarchy: hier,
    id: "cmpl-1",
    messages: [{ content }],
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

    const first = chunk("Agent/a", "hel");
    const second = chunk("Agent/a", "lo");
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
    response.push(chunk("Agent/a", "hi"));
    await settle();

    expect(mod.agentCompletion("Agent/a")).toHaveLength(1);
    expect(mod.agentCompletions().size).toBe(1);
    response.end();
  });

  it("ignores non-streaming spawns", async () => {
    harness.push(execution("agents/spawn", false, Promise.resolve("Agent/x")));
    harness.push(
      execution("agents/spawn", undefined, Promise.resolve("Agent/y")),
    );
    await settle();
    expect(mod.agentCompletions().size).toBe(0);
  });

  it("keeps the segment after the stream ends", async () => {
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await settle();
    response.push(chunk("Agent/a", "kept"));
    response.end();
    await settle();

    expect(mod.agentCompletion("Agent/a")).toHaveLength(1);
  });

  it("replaces the stored segment when a newer execution chunks the same AIH", async () => {
    const old = fakeResponse();
    harness.push(execution("agents/spawn", true, old));
    await settle();
    old.push(chunk("Agent/a", "old-1"));
    old.push(chunk("Agent/a", "old-2"));
    old.end();
    await settle();

    const fresh = fakeResponse();
    harness.push(execution("agents/spawn", true, fresh));
    await settle();
    const latest = chunk("Agent/a", "new-1");
    fresh.push(latest);
    await settle();

    // The whole old segment is gone — only the newest execution's
    // chunks remain.
    expect(mod.agentCompletion("Agent/a")).toEqual([latest]);
    fresh.end();
  });

  it("tracks concurrent executions' distinct AIHs independently", async () => {
    const one = fakeResponse();
    const two = fakeResponse();
    harness.push(execution("agents/spawn", true, one));
    harness.push(execution("agents/spawn", true, two));
    await settle();

    one.push(chunk("Agent/a", "a1"));
    two.push(chunk("Agent/b", "b1"));
    one.push(chunk("Agent/a", "a2"));
    await settle();

    expect(mod.agentCompletion("Agent/a")).toHaveLength(2);
    expect(mod.agentCompletion("Agent/b")).toHaveLength(1);
    one.end();
    two.end();
  });

  it("one execution replaces an AIH's segment only once, then appends", async () => {
    const response = fakeResponse();
    harness.push(execution("agents/spawn", true, response));
    await settle();

    response.push(chunk("Agent/a", "1"));
    await settle();
    response.push(chunk("Agent/a", "2"));
    response.push(chunk("Agent/a", "3"));
    await settle();

    expect(mod.agentCompletion("Agent/a")).toHaveLength(3);
    response.end();
  });
});

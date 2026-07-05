// @vitest-environment jsdom

import { describe, expect, it, beforeEach } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { CliCommandListenerExecution } from "@objectiveai/sdk";
import { useActiveAgents } from "./useActiveAgents";
import type { ListenerStream } from "./useListener";

/**
 * Tests for useActiveAgents: AIH refcounting over a fake
 * ListenerStream (React mount pattern; all infra faked).
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

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

/** A controllable fake ListenerStream. */
function fakeStream() {
  const queue: unknown[] = [];
  let ended = false;
  let wake: (() => void) | null = null;
  const stream = {
    push(run: unknown) {
      queue.push(run);
      wake?.();
    },
    endStream() {
      ended = true;
      wake?.();
    },
    next: async () => {
      for (;;) {
        if (queue.length > 0) {
          return { value: queue.shift() as CliCommandListenerExecution, done: false as const };
        }
        if (ended) return { value: undefined, done: true as const };
        await new Promise<void>((r) => {
          wake = r;
        });
        wake = null;
      }
    },
    return: async () => {
      ended = true;
      wake?.();
      return { value: undefined, done: true as const };
    },
    throw: async (e?: unknown) => {
      ended = true;
      wake?.();
      throw e;
    },
    [Symbol.asyncIterator]() {
      return stream;
    },
  };
  return stream;
}

function run(
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

function mountProbe(stream: ListenerStream) {
  let latest: string[] = [];
  function Probe() {
    latest = useActiveAgents(stream);
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

describe("useActiveAgents", () => {
  let stream: ReturnType<typeof fakeStream>;
  beforeEach(() => {
    stream = fakeStream();
  });

  it("counts an agents/spawn streaming run's string-Id AIH until its stream ends", async () => {
    const probe = mountProbe(stream as unknown as ListenerStream);
    const response = fakeResponse();
    stream.push(run("agents/spawn", true, response));
    await probe.settle();

    response.push({ some: "chunk" });
    response.push("Agent/root/1");
    await probe.settle();
    expect(probe.agents).toEqual(["Agent/root/1"]);

    response.end();
    await probe.settle();
    expect(probe.agents).toEqual([]);
    probe.unmount();
  });

  it("counts function executions' tagged announcements, not their string execution ids", async () => {
    const probe = mountProbe(stream as unknown as ListenerStream);
    const response = fakeResponse();
    stream.push(run("functions/execute/standard", true, response));
    await probe.settle();

    response.push("execution-id-123"); // NOT an AIH here
    response.push({
      type: "agent_instance_hierarchy",
      agent_instance_hierarchy: "Agent/a",
    });
    response.push({
      type: "agent_instance_hierarchy",
      agent_instance_hierarchy: "Agent/b",
    });
    await probe.settle();
    expect(probe.agents).toEqual(["Agent/a", "Agent/b"]);

    response.end();
    await probe.settle();
    expect(probe.agents).toEqual([]);
    probe.unmount();
  });

  it("ignores non-streaming runs and untracked paths", async () => {
    const probe = mountProbe(stream as unknown as ListenerStream);
    stream.push(run("agents/spawn", false, Promise.resolve("Agent/x")));
    stream.push(run("agents/spawn", undefined, Promise.resolve("Agent/y")));
    const other = fakeResponse();
    stream.push(run("plugins/run", true, other));
    other.push("Agent/z");
    await probe.settle();
    expect(probe.agents).toEqual([]);
    probe.unmount();
  });

  it("refcounts the same AIH across concurrent runs", async () => {
    const probe = mountProbe(stream as unknown as ListenerStream);
    const first = fakeResponse();
    const second = fakeResponse();
    stream.push(run("agents/spawn", true, first));
    stream.push(run("functions/execute/swiss_system", true, second));
    await probe.settle();

    first.push("Agent/shared");
    second.push({
      type: "agent_instance_hierarchy",
      agent_instance_hierarchy: "Agent/shared",
    });
    await probe.settle();
    expect(probe.agents).toEqual(["Agent/shared"]);

    first.end();
    await probe.settle();
    // Still held by the second run.
    expect(probe.agents).toEqual(["Agent/shared"]);

    second.end();
    await probe.settle();
    expect(probe.agents).toEqual([]);
    probe.unmount();
  });

  it("counts a duplicated announcement within one run only once", async () => {
    const probe = mountProbe(stream as unknown as ListenerStream);
    const response = fakeResponse();
    stream.push(run("agents/spawn", true, response));
    await probe.settle();

    response.push("Agent/dup");
    response.push("Agent/dup");
    await probe.settle();
    expect(probe.agents).toEqual(["Agent/dup"]);

    response.end();
    await probe.settle();
    expect(probe.agents).toEqual([]);
    probe.unmount();
  });

  it("keeps counting through in-band error items", async () => {
    const probe = mountProbe(stream as unknown as ListenerStream);
    const response = fakeResponse();
    stream.push(run("agents/spawn", true, response));
    await probe.settle();

    response.push({ type: "error", level: "warn", fatal: null, message: "hm" });
    response.push("Agent/resilient");
    await probe.settle();
    expect(probe.agents).toEqual(["Agent/resilient"]);
    response.end();
    await probe.settle();
    probe.unmount();
  });

  it("clears on unmount", async () => {
    const probe = mountProbe(stream as unknown as ListenerStream);
    const response = fakeResponse();
    stream.push(run("agents/spawn", true, response));
    await probe.settle();
    response.push("Agent/root/1");
    await probe.settle();
    expect(probe.agents).toEqual(["Agent/root/1"]);
    probe.unmount();
    // Post-unmount item delivery is a no-op (alive flag).
    response.push("Agent/root/2");
    response.end();
    await new Promise((r) => setTimeout(r, 0));
  });
});

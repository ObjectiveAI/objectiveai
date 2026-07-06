// @vitest-environment jsdom

import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import { act, createElement, useState } from "react";
import { createRoot, type Root } from "react-dom/client";

/**
 * Tests for useAgentLatestText: the custom db query for the newest
 * assistant text row (websocket executor + SDK dbQueryExecute
 * mocked), refreshed on active flips and polled every 60s while
 * active.
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const harness = vi.hoisted(() => ({
  /** The text the next queries return; null = no rows. */
  text: null as string | null,
  queries: [] as string[],
  reset() {
    harness.text = null;
    harness.queries = [];
  },
}));

vi.mock("../lib/websocket-executor", () => ({
  websocketExecutor: async () => ({}),
}));

vi.mock("@objectiveai/sdk", () => ({
  dbQueryExecute: async (_executor: unknown, request: { query: string }) => {
    harness.queries.push(request.query);
    return {
      command_tag: harness.text === null ? "SELECT 0" : "SELECT 1",
      columns: [{ name: "text", type: "TEXT" }],
      rows: harness.text === null ? [] : [[harness.text]],
      truncated: false,
    };
  },
}));

import { useAgentLatestText } from "./useAgentLatestText";

async function settle() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

async function settleFake() {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
  });
}

function mountProbe(hierarchy: string, active: boolean) {
  let latest: string | null = null;
  let setActive!: (next: boolean) => void;
  function Probe() {
    const [isActive, set] = useState(active);
    setActive = set;
    latest = useAgentLatestText(hierarchy, isActive);
    return null;
  }
  const container = document.createElement("div");
  const root: Root = createRoot(container);
  act(() => {
    root.render(createElement(Probe));
  });
  return {
    get text() {
      return latest;
    },
    flip(next: boolean) {
      act(() => {
        setActive(next);
      });
    },
    unmount: () =>
      act(() => {
        root.unmount();
      }),
  };
}

const AIH = "cli/me";

describe("useAgentLatestText", () => {
  beforeEach(() => {
    harness.reset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("fetches the newest text on mount with the AIH-scoped query", async () => {
    harness.text = "hello there";
    const probe = mountProbe(AIH, false);
    await settle();

    expect(probe.text).toBe("hello there");
    expect(harness.queries).toHaveLength(1);
    expect(harness.queries[0]).toContain(
      "assistant_response_content_text",
    );
    expect(harness.queries[0]).toContain(`'${AIH}'`);
    expect(harness.queries[0]).toContain("LIMIT 1");
    probe.unmount();
  });

  it("stays null when the agent has written nothing", async () => {
    const probe = mountProbe(AIH, false);
    await settle();
    expect(probe.text).toBeNull();
    probe.unmount();
  });

  it("polls every 60 seconds while active", async () => {
    vi.useFakeTimers();
    harness.text = "v1";
    const probe = mountProbe(AIH, true);
    await settleFake();
    expect(probe.text).toBe("v1");
    expect(harness.queries).toHaveLength(1);

    harness.text = "v2";
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(harness.queries).toHaveLength(2);
    expect(probe.text).toBe("v2");

    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(harness.queries).toHaveLength(3);
    probe.unmount();
  });

  it("does not poll while inactive", async () => {
    vi.useFakeTimers();
    harness.text = "static";
    const probe = mountProbe(AIH, false);
    await settleFake();
    expect(harness.queries).toHaveLength(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(180_000);
    });
    expect(harness.queries).toHaveLength(1);
    probe.unmount();
  });

  it("refreshes when the agent flips active to inactive", async () => {
    harness.text = "mid-run";
    const probe = mountProbe(AIH, true);
    await settle();
    expect(harness.queries).toHaveLength(1);

    harness.text = "final words";
    probe.flip(false);
    await settle();
    expect(harness.queries).toHaveLength(2);
    expect(probe.text).toBe("final words");
    probe.unmount();
  });

  it("escapes quotes in the hierarchy", async () => {
    const probe = mountProbe("cli/it's-me", false);
    await settle();
    expect(harness.queries[0]).toContain("'cli/it''s-me'");
    probe.unmount();
  });
});

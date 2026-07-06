// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";

/**
 * Tests for the ConversationModal: historical log rows from
 * useAgent (mocked), each ID-addressable part lazily fetched via
 * agents/logs/open (mocked, deferrable). jsdom has no
 * IntersectionObserver, so parts fetch immediately — the
 * on-screen gating is a browser-only refinement of the same start
 * path.
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const harness = vi.hoisted(() => ({
  logs: null as unknown[] | null,
  /** id → resolver queue for deferred opens. */
  openResolvers: new Map<number, (value: unknown) => void>(),
  openedIds: [] as number[],
  autoResolve: new Map<number, unknown>(),
  reset() {
    harness.logs = null;
    harness.openResolvers = new Map();
    harness.openedIds = [];
    harness.autoResolve = new Map();
  },
}));

vi.mock("../hooks/useAgent", () => ({
  useAgent: () => ({ logs: harness.logs, completions: [] }),
}));

vi.mock("../lib/websocket-executor", () => ({
  websocketExecutor: async () => ({}),
}));

vi.mock("@objectiveai/sdk", () => ({
  agentsLogsOpenExecute: (_executor: unknown, request: { id: number }) => {
    harness.openedIds.push(request.id);
    if (harness.autoResolve.has(request.id)) {
      return Promise.resolve(harness.autoResolve.get(request.id));
    }
    return new Promise((resolve) => {
      harness.openResolvers.set(request.id, resolve);
    });
  },
}));

import { ConversationModal } from "./ConversationModal";

function assistantRow(parts: unknown[]) {
  return {
    type: "assistant_response",
    agent_instance_hierarchy: "cli/me",
    response_id: "cmpl-1",
    parts,
  };
}

async function settle() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

function mount(onClose: () => void = () => {}) {
  const container = document.createElement("div");
  const root: Root = createRoot(container);
  act(() => {
    root.render(
      createElement(ConversationModal, { hierarchy: "cli/me", onClose }),
    );
  });
  return {
    container,
    unmount: () =>
      act(() => {
        root.unmount();
      }),
  };
}

describe("ConversationModal", () => {
  beforeEach(() => {
    harness.reset();
  });

  it("shows loading while logs are null, then empty for []", async () => {
    const view = mount();
    await settle();
    expect(
      view.container.querySelector("[data-conversation-loading]"),
    ).toBeTruthy();
    view.unmount();

    harness.logs = [];
    const empty = mount();
    await settle();
    expect(empty.container.textContent).toContain("no history");
    empty.unmount();
  });

  it("renders request rows as markers with no fetch", async () => {
    harness.logs = [
      {
        type: "agent_completion_request",
        agent_instance_hierarchy: "cli/me",
        delivered_at: "t",
        id: 1,
        response_id: "cmpl-1",
        sender_agent_instance_hierarchy: "cli",
      },
    ];
    const view = mount();
    await settle();
    const row = view.container.querySelector(
      '[data-log-row="agent_completion_request"]',
    );
    expect(row?.textContent).toContain("agent completion request");
    expect(row?.textContent).toContain("from cli");
    expect(harness.openedIds).toEqual([]);
    view.unmount();
  });

  it("fetches each part by id, loading dots until resolution", async () => {
    harness.logs = [
      assistantRow([
        { type: "text", id: 11, delivered_at: "t" },
        { type: "reasoning", id: 12, delivered_at: "t" },
      ]),
    ];
    const view = mount();
    await settle();
    expect(harness.openedIds.sort()).toEqual([11, 12]);
    expect(
      view.container.querySelectorAll("[data-part-loading]"),
    ).toHaveLength(2);

    await act(async () => {
      harness.openResolvers.get(11)?.({ type: "text", text: "hello *world*" });
    });
    await settle();
    const part = view.container.querySelector('[data-log-part="11"]');
    expect(part?.textContent).toContain("hello world");
    expect(part?.querySelector("em")?.textContent).toBe("world");
    // The other part still loads.
    expect(
      view.container.querySelectorAll("[data-part-loading]"),
    ).toHaveLength(1);
    view.unmount();
  });

  it("shows a failure line for CliError resolutions, others unaffected", async () => {
    harness.autoResolve.set(21, {
      type: "error",
      level: "error",
      fatal: null,
      message: "nope",
    });
    harness.autoResolve.set(22, { type: "text", text: "fine" });
    harness.logs = [
      assistantRow([
        { type: "text", id: 21, delivered_at: "t" },
        { type: "text", id: 22, delivered_at: "t" },
      ]),
    ];
    const view = mount();
    await settle();
    expect(
      view.container.querySelector('[data-log-part="21"]')?.textContent,
    ).toContain("failed to load");
    expect(
      view.container.querySelector('[data-log-part="22"]')?.textContent,
    ).toContain("fine");
    view.unmount();
  });

  it("labels tool calls, tool responses, and notifications", async () => {
    harness.autoResolve.set(31, { type: "text", text: "{}" });
    harness.autoResolve.set(32, { type: "text", text: "result" });
    harness.autoResolve.set(33, { type: "text", text: "ping" });
    harness.logs = [
      assistantRow([
        {
          type: "tool_call",
          id: 31,
          delivered_at: "t",
          function_name: "do_thing",
          tool_call_id: "call-9",
          tool_call_index: 0,
        },
      ]),
      {
        type: "tool_response",
        agent_instance_hierarchy: "cli/me",
        response_id: "cmpl-1",
        tool_call_id: "call-9",
        parts: [{ type: "text", id: 32, delivered_at: "t" }],
      },
      {
        type: "client_notification",
        agent_instance_hierarchy: "cli/me",
        response_id: "cmpl-1",
        queued_at: "t",
        sender_agent_instance_hierarchy: "cli/sender",
        parts: [{ type: "text", id: 33, delivered_at: "t" }],
      },
    ];
    const view = mount();
    await settle();
    expect(view.container.textContent).toContain("tool call do_thing (call-9)");
    const tool = view.container.querySelector('[data-log-row="tool_response"]');
    expect(tool?.textContent).toContain("call-9");
    expect(tool?.textContent).toContain("result");
    const note = view.container.querySelector(
      '[data-log-row="client_notification"]',
    );
    expect(note?.textContent).toContain("from cli/sender");
    expect(note?.textContent).toContain("ping");
    view.unmount();
  });

  it("closes via X, Escape, and backdrop — but not panel clicks", async () => {
    harness.logs = [];
    const onClose = vi.fn();
    const view = mount(onClose);
    await settle();

    (
      view.container.querySelector("[data-conversation-close]") as HTMLElement
    ).click();
    expect(onClose).toHaveBeenCalledTimes(1);

    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    });
    expect(onClose).toHaveBeenCalledTimes(2);

    act(() => {
      (
        view.container.querySelector("[data-conversation-modal]") as HTMLElement
      ).click();
    });
    expect(onClose).toHaveBeenCalledTimes(2);

    act(() => {
      (
        view.container.querySelector(
          "[data-conversation-backdrop]",
        ) as HTMLElement
      ).click();
    });
    expect(onClose).toHaveBeenCalledTimes(3);
    view.unmount();
  });
});

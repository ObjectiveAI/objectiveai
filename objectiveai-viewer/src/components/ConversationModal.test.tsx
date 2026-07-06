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

  /** A log-list agent_completion_request row. */
  function requestRow(id: number) {
    return {
      type: "agent_completion_request",
      agent_instance_hierarchy: "cli/me",
      delivered_at: "t",
      id,
      response_id: "cmpl-1",
      sender_agent_instance_hierarchy: "cli",
    };
  }

  it("unwraps a request into dated user + assistant messages", async () => {
    harness.autoResolve.set(1, {
      type: "agent_completion_request",
      created_at: 0,
      response_id: "cmpl-1",
      sender_agent_instance_hierarchy: "cli",
      body: {
        messages: [
          { role: "user", content: "hello there" },
          {
            role: "assistant",
            content: "hi *back*",
            reasoning: "thinking...",
          },
        ],
      },
    });
    harness.logs = [requestRow(1)];
    const view = mount();
    await settle();

    // Flattened: no request wrapper — messages are top-level, each
    // carrying the request's sender on its badge.
    expect(
      view.container.querySelector('[data-log-row="agent_completion_request"]'),
    ).toBeNull();
    const user = view.container.querySelector('[data-request-message="user"]');
    expect(user?.textContent).toContain("hello there");
    expect(user?.textContent).toContain("from cli");
    const assistant = view.container.querySelector(
      '[data-request-message="assistant"]',
    );
    expect(assistant?.textContent).toContain("hi back");
    expect(assistant?.querySelector("em")?.textContent).toBe("back");
    // Reasoning is collapsed; its text is inline (no fetch anywhere).
    expect(assistant?.querySelector("[data-reasoning-toggle]")).toBeTruthy();
    expect(assistant?.textContent).not.toContain("thinking...");
    expect(harness.openedIds).toEqual([1]);
    view.unmount();
  });

  it("renders inline assistant tool calls paired with their tool message", async () => {
    harness.autoResolve.set(2, {
      type: "agent_completion_request",
      created_at: 0,
      response_id: "cmpl-1",
      sender_agent_instance_hierarchy: "cli",
      body: {
        messages: [
          {
            role: "assistant",
            tool_calls: [
              {
                type: "function",
                id: "call-1",
                function: {
                  name: "do_thing",
                  arguments: '{"q":"x"}',
                },
              },
            ],
          },
          { role: "tool", tool_call_id: "call-1", content: "the result" },
        ],
      },
    });
    harness.logs = [requestRow(2)];
    const view = mount();
    await settle();

    const section = view.container.querySelector(
      '[data-tool-section="do_thing"]',
    );
    expect(section).toBeTruthy();
    act(() => {
      (section?.querySelector("[data-tool-toggle]") as HTMLElement).click();
    });
    await settle();
    // Inline: args pretty-print, the tool message is the response —
    // no id fetches beyond the request open itself.
    expect(section?.querySelector("[data-json-part]")?.textContent).toBe(
      JSON.stringify({ q: "x" }, null, 2),
    );
    expect(section?.textContent).toContain("the result");
    // The paired tool message does not also render standalone.
    expect(
      view.container.querySelectorAll('[data-tool-section]'),
    ).toHaveLength(1);
    expect(harness.openedIds).toEqual([2]);
    view.unmount();
  });

  it("renders an orphan tool message (call in a prior log) response-only", async () => {
    harness.autoResolve.set(3, {
      type: "agent_completion_request",
      created_at: 0,
      response_id: "cmpl-1",
      sender_agent_instance_hierarchy: "cli",
      body: {
        messages: [
          { role: "tool", tool_call_id: "call-GHOST", content: "orphan out" },
        ],
      },
    });
    harness.logs = [requestRow(3)];
    const view = mount();
    await settle();
    const section = view.container.querySelector(
      '[data-tool-section="tool response"]',
    );
    expect(section).toBeTruthy();
    act(() => {
      (section?.querySelector("[data-tool-toggle]") as HTMLElement).click();
    });
    await settle();
    expect(section?.textContent).toContain("orphan out");
    view.unmount();
  });

  it("renders inline image content without a fetch", async () => {
    harness.autoResolve.set(4, {
      type: "agent_completion_request",
      created_at: 0,
      response_id: "cmpl-1",
      sender_agent_instance_hierarchy: "cli",
      body: {
        messages: [
          {
            role: "user",
            content: [
              { type: "text", text: "look:" },
              {
                type: "image_url",
                image_url: { url: "data:image/png;base64,IMG" },
              },
            ],
          },
        ],
      },
    });
    harness.logs = [requestRow(4)];
    const view = mount();
    await settle();
    const img = view.container.querySelector("[data-media-image]");
    expect(img?.getAttribute("src")).toBe("data:image/png;base64,IMG");
    expect(harness.openedIds).toEqual([4]);
    view.unmount();
  });

  it("fetches each part by id, loading dots until resolution", async () => {
    harness.logs = [
      assistantRow([
        { type: "text", id: 11, delivered_at: "t" },
        { type: "text", id: 12, delivered_at: "t" },
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

  it("collapses reasoning by default; opening fetches, closing hides", async () => {
    harness.autoResolve.set(41, { type: "text", text: "deep thoughts" });
    harness.logs = [
      assistantRow([{ type: "reasoning", id: 41, delivered_at: "t" }]),
    ];
    const view = mount();
    await settle();
    // Collapsed: no body, no fetch.
    expect(view.container.querySelector('[data-log-part="41"]')).toBeNull();
    expect(harness.openedIds).toEqual([]);

    const toggle = view.container.querySelector(
      "[data-reasoning-toggle]",
    ) as HTMLElement;
    act(() => {
      toggle.click();
    });
    await settle();
    expect(harness.openedIds).toEqual([41]);
    expect(
      view.container.querySelector('[data-log-part="41"]')?.textContent,
    ).toContain("deep thoughts");

    act(() => {
      toggle.click();
    });
    expect(view.container.querySelector('[data-log-part="41"]')).toBeNull();
    view.unmount();
  });

  it("renders each media kind: image, video, audio, file", async () => {
    harness.autoResolve.set(61, {
      type: "image",
      url: "data:image/png;base64,IMG",
    });
    harness.autoResolve.set(62, {
      type: "video",
      url: "https://example.com/v.mp4",
    });
    harness.autoResolve.set(63, {
      type: "audio",
      data: "AUD",
      format: "mp3",
    });
    harness.autoResolve.set(64, {
      type: "file",
      file_url: "https://example.com/doc.pdf",
      filename: "doc.pdf",
    });
    harness.autoResolve.set(65, {
      type: "file",
      file_data: "Zmls",
      filename: "inline.bin",
    });
    harness.logs = [
      assistantRow([
        { type: "image", id: 61, delivered_at: "t" },
        { type: "video", id: 62, delivered_at: "t" },
        { type: "audio", id: 63, delivered_at: "t" },
        { type: "file", id: 64, delivered_at: "t" },
        { type: "file", id: 65, delivered_at: "t" },
      ]),
    ];
    const view = mount();
    await settle();

    const img = view.container.querySelector("[data-media-image]");
    expect(img?.getAttribute("src")).toBe("data:image/png;base64,IMG");
    expect(img?.className).toContain("max-h-80");

    const video = view.container.querySelector("[data-media-video]");
    expect(video?.getAttribute("src")).toBe("https://example.com/v.mp4");
    expect(video?.hasAttribute("controls")).toBe(true);

    // mp3 → audio/mpeg data URL.
    const audio = view.container.querySelector("[data-media-audio]");
    expect(audio?.getAttribute("src")).toBe("data:audio/mpeg;base64,AUD");
    expect(audio?.hasAttribute("controls")).toBe(true);

    const files = view.container.querySelectorAll("[data-media-file]");
    expect(files[0].getAttribute("href")).toBe("https://example.com/doc.pdf");
    expect(files[0].getAttribute("download")).toBe("doc.pdf");
    expect(files[0].textContent).toBe("doc.pdf");
    expect(files[1].getAttribute("href")).toBe(
      "data:application/octet-stream;base64,Zmls",
    );
    view.unmount();
  });

  it("falls back to a line when an image fails to load", async () => {
    harness.autoResolve.set(71, { type: "image", url: "http://bad/x.png" });
    harness.logs = [
      assistantRow([{ type: "image", id: 71, delivered_at: "t" }]),
    ];
    const view = mount();
    await settle();
    const img = view.container.querySelector(
      "[data-media-image]",
    ) as HTMLImageElement;
    act(() => {
      img.dispatchEvent(new Event("error"));
    });
    expect(
      view.container.querySelector("[data-media-image-failed]")?.textContent,
    ).toContain("image failed to load");
    expect(view.container.querySelector("[data-media-image]")).toBeNull();
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

  it("pairs tool calls with their responses as headed sections", async () => {
    harness.autoResolve.set(31, {
      type: "text",
      text: '{"query":"SELECT 1","limit":5}',
    });
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

    // One section, headed by the TOOL NAME, holding call args (pretty
    // JSON) and the paired response — no visible tool_call_id, no
    // separate tool row.
    const section = view.container.querySelector(
      '[data-tool-section="do_thing"]',
    );
    expect(section).toBeTruthy();
    expect(section?.textContent).toContain("do_thing");
    // Closed by default: header only, no fetches yet.
    expect(section?.querySelector("[data-json-part]")).toBeNull();
    expect(harness.openedIds).toEqual([33]);
    act(() => {
      (section?.querySelector("[data-tool-toggle]") as HTMLElement).click();
    });
    await settle();
    expect(section?.textContent).not.toContain("call-9");
    expect(section?.querySelector("[data-json-part]")?.textContent).toBe(
      JSON.stringify({ query: "SELECT 1", limit: 5 }, null, 2),
    );
    expect(section?.textContent).toContain("result");
    expect(
      view.container.querySelector('[data-log-row="tool_response"]'),
    ).toBeNull();

    const note = view.container.querySelector(
      '[data-log-row="client_notification"]',
    );
    expect(note?.textContent).toContain("from cli/sender");
    expect(note?.textContent).toContain("ping");
    view.unmount();
  });

  it("omits the response area for unanswered calls; orphan responses vanish", async () => {
    harness.autoResolve.set(51, { type: "text", text: "{}" });
    harness.logs = [
      assistantRow([
        {
          type: "tool_call",
          id: 51,
          delivered_at: "t",
          function_name: "pending_tool",
          tool_call_id: "call-1",
          tool_call_index: 0,
        },
      ]),
      {
        type: "tool_response",
        agent_instance_hierarchy: "cli/me",
        response_id: "cmpl-1",
        tool_call_id: "call-GHOST",
        parts: [{ type: "text", id: 52, delivered_at: "t" }],
      },
    ];
    const view = mount();
    await settle();

    const section = view.container.querySelector(
      '[data-tool-section="pending_tool"]',
    );
    act(() => {
      (section?.querySelector("[data-tool-toggle]") as HTMLElement).click();
    });
    await settle();
    expect(section?.textContent).toContain("call");
    expect(section?.textContent).not.toContain("response");
    // The orphan response fetches nothing and renders nowhere.
    expect(harness.openedIds).toEqual([51]);
    expect(view.container.textContent).not.toContain("call-GHOST");
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

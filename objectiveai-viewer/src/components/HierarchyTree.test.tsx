// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import type { AgentStatus } from "../hooks/useAgents";

/**
 * Tests for the HierarchyTree: hierarchies split on `/` into nested
 * per-segment nodes, each showing ONLY its own segment; nodes that
 * are agents (active/inactive) versus pure structure are marked; a
 * branch can itself be an agent.
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const harness = vi.hoisted(() => ({
  invokes: [] as Array<[string, unknown]>,
  agents: [] as AgentStatus[],
  tags: new Map<string, string[]>(),
  tagsLoading: false,
  definitions: new Map<string, unknown>(),
  definitionLoading: false,
  latestTexts: new Map<string, string>(),
}));

vi.mock("../lib/tauri", () => ({
  tauriInvoke: (cmd: string, args: unknown) => {
    harness.invokes.push([cmd, args]);
    return Promise.resolve(undefined);
  },
}));

vi.mock("../hooks/useAgents", () => ({
  useAgents: () => harness.agents,
}));

vi.mock("../hooks/useAgentInstanceTags", () => ({
  useAgentInstanceTags: (hierarchy: string) => ({
    tags: harness.tags.get(hierarchy) ?? [],
    loading: harness.tagsLoading,
  }),
}));

vi.mock("../hooks/useAgentDefinition", () => ({
  useAgentDefinition: (hierarchy: string) => ({
    agent: harness.definitions.get(hierarchy) ?? null,
    loading: harness.definitionLoading,
  }),
}));

vi.mock("../hooks/useAgentLatestText", () => ({
  useAgentLatestText: (hierarchy: string) =>
    harness.latestTexts.get(hierarchy) ?? null,
}));

import { HierarchyTree } from "./HierarchyTree";

function agent(
  hier: string,
  active: boolean,
  times?: { created_at?: string | null; last_active_at?: string | null },
): AgentStatus {
  return {
    agent_instance_hierarchy: hier,
    active,
    created_at: times?.created_at ?? null,
    last_active_at: times?.last_active_at ?? null,
  };
}

function render(
  agents: AgentStatus[],
  opts?: {
    tags?: Record<string, string[]>;
    tagsLoading?: boolean;
    definitions?: Record<string, unknown>;
    definitionLoading?: boolean;
    latestTexts?: Record<string, string>;
  },
) {
  harness.agents = agents;
  harness.tags = new Map(Object.entries(opts?.tags ?? {}));
  harness.tagsLoading = opts?.tagsLoading ?? false;
  harness.definitions = new Map(Object.entries(opts?.definitions ?? {}));
  harness.definitionLoading = opts?.definitionLoading ?? false;
  harness.latestTexts = new Map(Object.entries(opts?.latestTexts ?? {}));
  harness.invokes = [];
  const container = document.createElement("div");
  const root = createRoot(container);
  act(() => {
    root.render(createElement(HierarchyTree));
  });
  return {
    container,
    nodes: () =>
      [...container.querySelectorAll<HTMLElement>("[data-node-kind]")].map(
        (el) => ({
          name: el.dataset.nodeName,
          kind: el.dataset.nodeKind,
        }),
      ),
    unmount: () =>
      act(() => {
        root.unmount();
      }),
  };
}

describe("HierarchyTree", () => {
  it("splits hierarchies into per-segment nodes, one per shared prefix", () => {
    const view = render([
      agent("cli/foo/bar", false),
      agent("cli/foo/buzz", true),
    ]);

    expect(view.nodes()).toEqual([
      { name: "cli", kind: "branch" },
      { name: "foo", kind: "branch" },
      { name: "bar", kind: "agent-inactive" },
      { name: "buzz", kind: "agent-active" },
    ]);
    view.unmount();
  });

  it("never renders a full hierarchy string", () => {
    const view = render([agent("cli/foo/bar", true)]);
    expect(view.container.textContent).toContain("bar");
    expect(view.container.textContent).not.toContain("cli/foo/bar");
    expect(view.container.textContent).not.toContain("cli/foo");
    view.unmount();
  });

  it("marks a branch that is itself an agent", () => {
    const view = render([
      agent("cli/foo", true),
      agent("cli/foo/bar", false),
    ]);

    expect(view.nodes()).toEqual([
      { name: "cli", kind: "branch" },
      { name: "foo", kind: "agent-active" },
      { name: "bar", kind: "agent-inactive" },
    ]);
    view.unmount();
  });

  it("nests children under their parent node, not beside it", () => {
    const view = render([
      agent("cli/foo/bar", false),
      agent("cli/other", false),
    ]);

    const cli = view.container.querySelector('[data-node-name="cli"]');
    // box -> card-pair wrapper -> node cell (which holds the
    // children row).
    const parent = cli?.parentElement?.parentElement;
    expect(parent?.querySelector('[data-node-name="bar"]')).toBeTruthy();
    expect(parent?.querySelector('[data-node-name="other"]')).toBeTruthy();
    // Only ONE cli node exists — shared prefixes collapse.
    expect(
      view.container.querySelectorAll('[data-node-name="cli"]'),
    ).toHaveLength(1);
    view.unmount();
  });

  it("renders single-segment hierarchies as root agents", () => {
    const view = render([agent("Viewer", true)]);
    expect(view.nodes()).toEqual([{ name: "Viewer", kind: "agent-active" }]);
    view.unmount();
  });

  it("shows locale-formatted spawn and last-active times on agent boxes", () => {
    const spawned = "2026-06-20T00:00:00+00:00";
    const lastActive = "2026-07-05T01:07:14+00:00";
    const view = render([
      agent("cli/timed", true, {
        created_at: spawned,
        last_active_at: lastActive,
      }),
    ]);

    const box = view.container.querySelector('[data-node-name="timed"]');
    expect(box?.textContent).toContain("spawned");
    expect(box?.textContent).toContain(new Date(spawned).toLocaleString());
    expect(box?.textContent).toContain("active");
    expect(box?.textContent).toContain(
      new Date(lastActive).toLocaleString(),
    );
    // The structural parent carries no times.
    const cli = view.container.querySelector('[data-node-name="cli"]');
    expect(cli?.textContent).not.toContain("spawned");
    view.unmount();
  });

  it("shows an em dash for unknown times", () => {
    const view = render([agent("solo", false)]);
    const box = view.container.querySelector('[data-node-name="solo"]');
    expect(box?.textContent).toContain("spawned");
    expect(box?.textContent).toContain("—");
    view.unmount();
  });

  it("lays siblings out left-to-right in one shared row", () => {
    const view = render([
      agent("cli/foo/bar", false),
      agent("cli/foo/buzz", false),
    ]);
    const bar = view.container.querySelector('[data-node-name="bar"]');
    const buzz = view.container.querySelector('[data-node-name="buzz"]');
    // Same tier: both node cells (box -> card-pair wrapper -> node
    // cell -> content wrapper -> child cell) share one children-row
    // container.
    expect(
      bar?.parentElement?.parentElement?.parentElement?.parentElement
        ?.parentElement,
    ).toBe(
      buzz?.parentElement?.parentElement?.parentElement?.parentElement
        ?.parentElement,
    );
    view.unmount();
  });

  it("renders each tag in its own box on the agent node", () => {
    const view = render([agent("cli/tagged", false)], {
      tags: { "cli/tagged": ["rick-sanchez", "lebowski"] },
    });
    const box = view.container.querySelector('[data-node-name="tagged"]');
    const chips = [...(box?.querySelectorAll("[data-tag]") ?? [])];
    expect(chips.map((chip) => chip.textContent)).toEqual([
      "rick-sanchez",
      "lebowski",
    ]);
    // The AIH segment rides the row as just another chip, first.
    const aihChip = box?.querySelector("[data-tag-aih]");
    expect(aihChip?.textContent).toBe("tagged");
    expect(aihChip?.className).toBe(chips[0].className);
    // The structural parent's own box carries no tags.
    expect(
      view.container
        .querySelector('[data-node-name="cli"]')
        ?.querySelector("[data-tag]") ?? null,
    ).toBeNull();
    view.unmount();
  });

  it("shows the animated ellipsis while tags load", () => {
    const view = render([agent("cli/waiting", true)], { tagsLoading: true });
    const box = view.container.querySelector('[data-node-name="waiting"]');
    const ellipsis = box?.querySelector("[data-tags-loading]");
    expect(ellipsis).toBeTruthy();
    expect(box?.querySelectorAll("[data-tag]")).toHaveLength(0);
    // Three dots, pulse-animated, staggered so the ellipsis actually
    // ripples.
    const dots = [...(ellipsis?.children ?? [])];
    expect(dots).toHaveLength(3);
    for (const dot of dots) {
      expect(dot.className).toContain("animate-pulse");
    }
    expect(dots[1].className).toContain("[animation-delay:300ms]");
    expect(dots[2].className).toContain("[animation-delay:600ms]");
    view.unmount();
  });

  it("shows an INLINE definition as pretty JSON, nulls omitted", () => {
    const definition = {
      upstream: "mock",
      output_mode: "instruction",
      fallbacks: null,
    };
    const view = render([agent("cli/defined", false)], {
      definitions: { "cli/defined": definition },
    });
    const pre = view.container.querySelector("[data-agent-definition]");
    // Null-valued keys (fallbacks) are omitted from the pretty print.
    expect(pre?.textContent).toBe(
      JSON.stringify(
        { upstream: "mock", output_mode: "instruction" },
        null,
        2,
      ),
    );
    view.unmount();
  });

  it("renders a github remote as badge + repo link with the short commit", () => {
    const view = render([agent("cli/gh", false)], {
      definitions: {
        "cli/gh": {
          remote: "github",
          owner: "ObjectiveAI",
          repository: "foo",
          commit: "3f2a9c1deadbeef",
        },
      },
    });
    const row = view.container.querySelector('[data-agent-remote="github"]');
    expect(row?.textContent).toContain("github");
    expect(row?.textContent).toContain("ObjectiveAI/foo");
    expect(row?.textContent).toContain("@3f2a9c1");
    (row?.querySelector("[data-remote-link]") as HTMLElement).click();
    expect(harness.invokes).toEqual([
      [
        "open_url",
        { url: "https://github.com/ObjectiveAI/foo/tree/3f2a9c1deadbeef" },
      ],
    ]);
    view.unmount();
  });

  it("renders a client remote linking to the local agents folder", () => {
    const view = render([agent("cli/local", false)], {
      definitions: {
        "cli/local": {
          remote: "client",
          owner: "ObjectiveAI",
          repository: "rick-sanchez",
          commit: null,
        },
      },
    });
    const row = view.container.querySelector('[data-agent-remote="client"]');
    expect(row?.textContent).toContain("client");
    (row?.querySelector("[data-remote-link]") as HTMLElement).click();
    expect(harness.invokes).toEqual([
      [
        "open_agent_remote",
        { owner: "ObjectiveAI", repository: "rick-sanchez" },
      ],
    ]);
    // No JSON block for remotes.
    expect(view.container.querySelector("[data-agent-definition]")).toBeNull();
    view.unmount();
  });

  it("renders a mock remote as plain text — no link", () => {
    const view = render([agent("cli/mocked", false)], {
      definitions: { "cli/mocked": { remote: "mock", name: "test-thing" } },
    });
    const row = view.container.querySelector('[data-agent-remote="mock"]');
    expect(row?.textContent).toContain("mock");
    expect(row?.textContent).toContain("test-thing");
    expect(row?.querySelector("[data-remote-link]")).toBeNull();
    expect(harness.invokes).toEqual([]);
    view.unmount();
  });

  it("shows its own loading dots for the definition; nothing when unrecorded", () => {
    const loadingView = render([agent("cli/waiting", false)], {
      definitionLoading: true,
    });
    expect(
      loadingView.container.querySelector("[data-definition-loading]"),
    ).toBeTruthy();
    loadingView.unmount();

    const bareView = render([agent("cli/bare", false)]);
    expect(
      bareView.container.querySelector("[data-agent-definition]"),
    ).toBeNull();
    expect(
      bareView.container.querySelector("[data-definition-loading]"),
    ).toBeNull();
    bareView.unmount();
  });

  it("shows the latest assistant text in its own box below the agent's; none when unwritten", () => {
    const view = render(
      [agent("cli/wrote", false), agent("cli/silent", false)],
      { latestTexts: { "cli/wrote": "Both cleared — *BURRP*" } },
    );
    const boxes = view.container.querySelectorAll("[data-latest-text]");
    expect(boxes).toHaveLength(1);
    expect(boxes[0].textContent).toBe("Both cleared — *BURRP*");
    // Below the agent's own box, as a SIBLING in the node cell — not
    // inside the agent box.
    const agentBox = view.container.querySelector(
      '[data-node-name="wrote"]',
    );
    expect(agentBox?.querySelector("[data-latest-text]")).toBeNull();
    expect(agentBox?.parentElement?.querySelector("[data-latest-text]")).toBe(
      boxes[0],
    );
    view.unmount();
  });

  it("shows no expand toggle when the message fits", () => {
    const view = render([agent("cli/short", false)], {
      latestTexts: { "cli/short": "one-liner" },
    });
    expect(view.container.querySelector("[data-expand-text]")).toBeNull();
    view.unmount();
  });

  it("shows the expand toggle only for clipped content, and toggles", () => {
    // jsdom has no layout: fake an overflowing clip element.
    const original = Object.getOwnPropertyDescriptor(
      HTMLElement.prototype,
      "scrollHeight",
    );
    Object.defineProperty(HTMLElement.prototype, "scrollHeight", {
      configurable: true,
      get() {
        return 500;
      },
    });
    try {
      const view = render([agent("cli/talker", false)], {
        latestTexts: { "cli/talker": "long ".repeat(200) },
      });
      const box = view.container.querySelector("[data-latest-text]");
      const clip = box?.firstElementChild;
      expect(clip?.className).toContain("max-h-[77px]");

      const toggle = box?.querySelector(
        "[data-expand-text]",
      ) as HTMLElement | null;
      expect(toggle).toBeTruthy();

      act(() => {
        toggle?.click();
      });
      // Expanded: the clamp is gone, toggle stays to collapse.
      const expandedClip =
        view.container.querySelector("[data-latest-text]")?.firstElementChild;
      expect(expandedClip?.className).not.toContain("max-h-[77px]");
      expect(
        view.container.querySelector("[data-expand-text]"),
      ).toBeTruthy();

      act(() => {
        (
          view.container.querySelector("[data-expand-text]") as HTMLElement
        ).click();
      });
      const collapsedClip =
        view.container.querySelector("[data-latest-text]")?.firstElementChild;
      expect(collapsedClip?.className).toContain("max-h-[77px]");
      view.unmount();
    } finally {
      if (original) {
        Object.defineProperty(HTMLElement.prototype, "scrollHeight", original);
      }
    }
  });

  it("renders nothing for no agents (the watermark shows through)", () => {
    const view = render([]);
    expect(view.nodes()).toEqual([]);
    view.unmount();
  });
});

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
  agents: [] as AgentStatus[],
}));

vi.mock("../hooks/useAgents", () => ({
  useAgents: () => harness.agents,
}));

import { HierarchyTree } from "./HierarchyTree";

function agent(hier: string, active: boolean): AgentStatus {
  return { agent_instance_hierarchy: hier, active, created_at: null };
}

function render(agents: AgentStatus[]) {
  harness.agents = agents;
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
    const parent = cli?.parentElement;
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

  it("renders nothing for no agents (the watermark shows through)", () => {
    const view = render([]);
    expect(view.nodes()).toEqual([]);
    view.unmount();
  });
});

// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";

/**
 * Tests for useAgentDefinition: the one-off `agents instances get`
 * read of the recorded agent definition (websocket executor + SDK
 * execute mocked). No live half — definitions only change on
 * respawn-by-spec.
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const harness = vi.hoisted(() => ({
  instanceItems: [] as unknown[],
  instanceRequests: [] as unknown[],
  instancesGate: null as (() => void) | null,
  gateInstances: false,
  failConnect: false,
  reset() {
    harness.instanceItems = [];
    harness.instanceRequests = [];
    harness.instancesGate = null;
    harness.gateInstances = false;
    harness.failConnect = false;
  },
}));

vi.mock("../lib/sse-executor", () => ({
  sseExecutor: async () => {
    if (harness.failConnect) throw new Error("daemon unavailable");
    return {
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
    };
  },
}));

vi.mock("@objectiveai/sdk", () => ({
  // Passthrough: the executor's items ARE the typed items.
  agentsInstancesGetExecute: (
    executor: { execute: (request: unknown) => AsyncIterable<unknown> },
    request: unknown,
  ) => executor.execute(request),
}));

import {
  useAgentDefinition,
  type AgentDefinitionResult,
} from "./useAgentDefinition";

const REMOTE = {
  remote: "client",
  owner: "ObjectiveAI",
  repository: "rick-sanchez",
  commit: null,
};

function instanceItem(hier: string, agent: unknown) {
  return {
    agent_instance_hierarchy: hier,
    created_at: null,
    last_active_at: null,
    logged: 0,
    queued: 0,
    tags: [],
    ...(agent === undefined ? {} : { agent }),
  };
}

async function settle() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

function mountProbe(hierarchy: string) {
  let latest: AgentDefinitionResult = { agent: null, loading: true };
  function Probe() {
    latest = useAgentDefinition(hierarchy);
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

const AIH = "cli/me";

describe("useAgentDefinition", () => {
  beforeEach(() => {
    harness.reset();
  });

  it("loads the matching item's agent, targeted at the split AIH", async () => {
    harness.instanceItems = [
      { type: "error", level: "warn", fatal: null, message: "hm" },
      instanceItem("cli/other", { remote: "x" }),
      instanceItem(AIH, REMOTE),
    ];
    const probe = mountProbe(AIH);
    await settle();

    expect(probe.result).toEqual({ agent: REMOTE, loading: false });
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

  it("is loading (agent null) until the read completes", async () => {
    harness.gateInstances = true;
    harness.instanceItems = [instanceItem(AIH, REMOTE)];
    const probe = mountProbe(AIH);
    await settle();
    expect(probe.result).toEqual({ agent: null, loading: true });

    act(() => harness.instancesGate?.());
    await settle();
    expect(probe.result).toEqual({ agent: REMOTE, loading: false });
    probe.unmount();
  });

  it("resolves to null when the item carries no agent", async () => {
    harness.instanceItems = [instanceItem(AIH, undefined)];
    const probe = mountProbe(AIH);
    await settle();
    expect(probe.result).toEqual({ agent: null, loading: false });
    probe.unmount();
  });

  it("ends loading with null when the daemon is unreachable", async () => {
    harness.failConnect = true;
    const probe = mountProbe(AIH);
    await settle();
    expect(probe.result).toEqual({ agent: null, loading: false });
    probe.unmount();
  });
});

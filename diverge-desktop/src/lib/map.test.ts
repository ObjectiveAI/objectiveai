import { describe, expect, it } from "vitest";
import type { AgentView } from "../bindings/AgentView";
import type { LogEntry } from "../bindings/LogEntry";
import type { LogItem } from "../bindings/LogItem";
import type { MapNode } from "../bindings/MapNode";
import { agentsIn, doing, places, step, tailFrom } from "./map";
import { turns } from "./tails";

function agent(key: string, over: Partial<AgentView> = {}): AgentView {
  return {
    key, name: key, template: "t", index: 1, creator: { kind: "client", identity: "you" }, tags: [], image_name: "diverge-agentic-loop-cc", digest: "",
    created: "2026-10-15T08:00:00Z", active: false, last_active: null, provider: null, logs_index: 0, tools: [], ...over,
  };
}
const node = (key: string, children: MapNode[] = [], over: Partial<AgentView> = {}): MapNode => ({ key: `agent:${key}`, what: { kind: "agent", agent: agent(key, over), cards: 0 }, working: !!over.active, children });

// site-fixes made links and a checker; links made one more. A tool made the digest.
const tree: MapNode[] = [
  node("site-fixes", [node("links", [node("deep")]), node("checker")], { active: true }),
  { key: "tool:tt#1", what: { kind: "tool", maker: { kind: "tool", template: "tt", index: 1, name: "nightly-digest" } }, working: false, children: [node("digest")] },
  node("notes"),
];

describe("the map's arrow keys", () => {
  it("go to what an agent made, back to its maker, and across to its neighbours, along the way the map grows", () => {
    // Down: what it made is below, its neighbours beside it.
    expect(step(tree, "agent:site-fixes", "ArrowDown", "down")).toBe("agent:links");
    expect(step(tree, "agent:links", "ArrowRight", "down")).toBe("agent:checker");
    expect(step(tree, "agent:checker", "ArrowLeft", "down")).toBe("agent:links");
    expect(step(tree, "agent:deep", "ArrowUp", "down")).toBe("agent:links");
    expect(step(tree, "agent:site-fixes", "ArrowRight", "down")).toBe("tool:tt#1");
    // Across: the same moves, turned.
    expect(step(tree, "agent:site-fixes", "ArrowRight", "across")).toBe("agent:links");
    expect(step(tree, "agent:links", "ArrowDown", "across")).toBe("agent:checker");
    expect(step(tree, "agent:deep", "ArrowLeft", "across")).toBe("agent:links");
    expect(step(tree, "tool:tt#1", "ArrowRight", "across")).toBe("agent:digest");
  });

  it("stop at the edges, and Home and End go to the first and the last", () => {
    expect(step(tree, "agent:notes", "ArrowRight", "down")).toBeNull();
    expect(step(tree, "agent:site-fixes", "ArrowUp", "down")).toBeNull();
    expect(step(tree, "agent:deep", "ArrowDown", "down")).toBeNull();
    expect(step(tree, "agent:deep", "Home", "down")).toBe("agent:site-fixes");
    expect(step(tree, "agent:site-fixes", "End", "down")).toBe("agent:notes");
    expect(step(tree, "agent:site-fixes", "a", "down")).toBeNull();
    expect(step([], null, "ArrowDown", "down")).toBeNull();
  });

  it("start from the first card when nothing has focus", () => {
    expect(step(tree, null, "ArrowDown", "down")).toBe("agent:site-fixes");
    expect(step(tree, "agent:gone", "ArrowRight", "across")).toBe("agent:site-fixes");
  });
});

describe("the map's places", () => {
  it("lists every node once, makers before what they made", () => {
    const { order, at } = places(tree);
    expect(order).toEqual(["agent:site-fixes", "agent:links", "agent:deep", "agent:checker", "tool:tt#1", "agent:digest", "agent:notes"]);
    expect(at.get("agent:deep")).toEqual({ parent: "agent:links", siblings: ["agent:deep"], children: [] });
    expect(agentsIn(tree)).toEqual(["site-fixes", "links", "deep", "checker", "digest", "notes"]);
  });

  it("reads a card's tail from sixty items back, never before the start", () => {
    expect(tailFrom(500)).toBe(440);
    expect(tailFrom(12)).toBe(0);
  });
});

let n = 0;
const e = (item: LogItem): LogEntry => ({ logs_index: ++n, created: "2026-10-15T08:00:00Z", sender: null, item });
const here = { kind: "outgoing" as const, address: "127.0.0.1:4640" };

describe("what a card says an agent is doing", () => {
  const run: LogEntry[] = [
    e({ kind: "user_text", key: "m1", text: "Tidy the site" }),
    e({ kind: "active", provider: here }),
    e({ kind: "tool_call", parent: null, id: "c1", name: "Read", arguments: "{\"file_path\":\"/site/index.html\"}" }),
    e({ kind: "tool_response", parent: null, id: "c1", is_error: false, text: "<html>" }),
    e({ kind: "tool_call", parent: null, id: "task", name: "Task", arguments: "{\"description\":\"Check every link\"}" }),
    e({ kind: "reasoning", parent: "task", text: "Four links. " }),
    e({ kind: "tool_call", parent: "task", id: "c2", name: "Bash", arguments: "{\"command\":\"ls /site/old-page\"}" }),
  ];

  it("while it works: the step it's on, inside a helper if that's where the work is, and the helper with its latest line", () => {
    const now = doing(run, true);
    expect(now.running).toBe(true);
    expect(now.line).toBe("Bash: ls /site/old-page");
    expect(now.helpers).toEqual([{ id: "task", name: "Task", line: "Bash: ls /site/old-page", done: false }]);
  });

  it("once the run ends: what it said last, and its helpers fold to a count", () => {
    const ended = [
      ...run,
      e({ kind: "tool_response", parent: "task", id: "c2", is_error: true, text: "No such file" }),
      e({ kind: "tool_response", parent: null, id: "task", is_error: false, text: "One broken" }),
      e({ kind: "text", parent: null, text: "**One link** is broken:\n/old-page." }),
      e({ kind: "inactive", provider: here }),
    ];
    const last = doing(ended, false);
    expect(last.running).toBe(false);
    expect(last.line).toBe("One link is broken: /old-page.");
    expect(last.helpers.map((h) => [h.name, h.done])).toEqual([["Task", true]]);
  });

  it("a run that went wrong says so; a list that says it stopped is believed over a tail that hasn't caught up", () => {
    const broken = [e({ kind: "active", provider: here }), e({ kind: "reasoning", parent: null, text: "Reading the list." }), e({ kind: "error", message: "no API key" }), e({ kind: "inactive", provider: here })];
    expect(doing(broken, false).line).toMatch(/no API key$/);
    expect(doing(run, false).running).toBe(false);
    expect(doing([], false)).toEqual({ line: null, helpers: [], running: false });
  });
});

describe("the map's log readers", () => {
  it("open no more than the cap at once; the rest wait their turn, in order", async () => {
    const q = turns(2);
    const order: string[] = [];
    const a = q.take(), b = q.take(), c = q.take();
    a.ready.then(() => order.push("a"));
    b.ready.then(() => order.push("b"));
    c.ready.then(() => order.push("c"));
    await Promise.resolve();
    expect(order).toEqual(["a", "b"]);
    expect(q.counts()).toEqual({ open: 2, waiting: 1 });
    a.done();
    a.done(); // twice does nothing
    await Promise.resolve();
    expect(order).toEqual(["a", "b", "c"]);
    expect(q.counts()).toEqual({ open: 2, waiting: 0 });
  });

  it("let a card that leaves the queue before its turn go, and give its turn to the next", async () => {
    const q = turns(1);
    const a = q.take(), b = q.take(), c = q.take();
    b.done();
    a.done();
    await c.ready;
    expect(q.counts()).toEqual({ open: 1, waiting: 0 });
  });
});

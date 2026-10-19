import { describe, expect, it } from "vitest";
import type { LogEntry } from "../bindings/LogEntry";
import type { LogItem } from "../bindings/LogItem";
import { fold, shown } from "./conversation";

let i = 0;
const e = (item: LogItem): LogEntry => ({ logs_index: ++i, created: "2026-09-25T12:00:00Z", item });
const here = { kind: "outgoing" as const, address: "127.0.0.1:4640" };

describe("fold", () => {
  it("stitches text pieces, pairs tools, nests a helper, and keeps markers", () => {
    const blocks = fold([
      e({ kind: "user_text", key: "m1", text: "check the site" }),
      e({ kind: "active", provider: here }),
      e({ kind: "reasoning", parent: null, text: "Look " }),
      e({ kind: "reasoning", parent: null, text: "first." }),
      e({ kind: "tool_call", parent: null, id: "call_1", name: "Task", arguments: "{\"description\":\"links\"}" }),
      e({ kind: "text", parent: "call_1", text: "One is " }),
      e({ kind: "text", parent: "call_1", text: "broken." }),
      e({ kind: "tool_response", parent: null, id: "call_1", is_error: false, text: "1 broken" }),
      e({ kind: "text", parent: null, text: "Done " }),
      e({ kind: "text", parent: null, text: "here." }),
      e({ kind: "usage", prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 }),
      e({ kind: "inactive", provider: here }),
    ]);
    expect(blocks.map((b) => b.kind)).toEqual(["user", "started", "turn", "finished"]);
    const turn = blocks[2];
    if (turn.kind !== "turn") throw new Error("expected a turn");
    expect(turn.parts[0]).toMatchObject({ kind: "reasoning", text: "Look first." });
    const tool = turn.parts[1];
    if (tool.kind !== "tool") throw new Error("expected a tool");
    expect(tool.answer).toMatchObject({ text: "1 broken", isError: false });
    expect(tool.parts).toEqual([{ kind: "text", text: "One is broken." }]);
    expect(turn.parts[2]).toEqual({ kind: "text", text: "Done here." });
    expect(turn.usage?.total).toBe(15);
  });

  it("does not merge a new run's tool calls into an old run's, even with the same id", () => {
    const run = () => [
      e({ kind: "active", provider: here }),
      e({ kind: "tool_call", parent: null, id: "call_1", name: "Read", arguments: "{}" }),
      e({ kind: "tool_response", parent: null, id: "call_1", is_error: false, text: "ok" }),
      e({ kind: "inactive", provider: here }),
    ];
    const blocks = fold([...run(), ...run()]);
    const turns = blocks.filter((b) => b.kind === "turn");
    expect(turns).toHaveLength(2);
    for (const turn of turns) if (turn.kind === "turn") expect(turn.parts).toHaveLength(1);
  });

  it("folds the work between two things the agent said into one line", () => {
    const blocks = fold([
      e({ kind: "active", provider: here }),
      e({ kind: "reasoning", parent: null, text: "Look first." }),
      e({ kind: "tool_call", parent: null, id: "a", name: "Read", arguments: "{}" }),
      e({ kind: "tool_response", parent: null, id: "a", is_error: false, text: "ok" }),
      e({ kind: "tool_call", parent: null, id: "b", name: "Read", arguments: "{}" }),
      e({ kind: "tool_response", parent: null, id: "b", is_error: true, text: "no" }),
      e({ kind: "text", parent: null, text: "Pass 1 done." }),
      e({ kind: "reasoning", parent: null, text: "Next." }),
      e({ kind: "tool_call", parent: null, id: "c", name: "Bash", arguments: "{}" }),
    ]);
    const turn = blocks.find((b) => b.kind === "turn");
    if (!turn || turn.kind !== "turn") throw new Error("expected a turn");
    const view = shown(turn.parts);
    expect(view.map((v) => v.kind)).toEqual(["work", "say", "work"]);
    const first = view[0];
    if (first.kind !== "work") throw new Error("expected work");
    expect(first.summary.tools).toEqual([{ name: "Read", count: 2 }]);
    expect(first.summary.thought).toBe(true);
    expect(first.summary.failed).toBe(1);
    const lastGroup = view[2];
    if (lastGroup.kind !== "work") throw new Error("expected work");
    expect(lastGroup.summary.waiting).toBe(true);
    expect(lastGroup.summary.last?.name).toBe("Bash");
  });

  it("shows an error where it happened", () => {
    const blocks = fold([e({ kind: "error", message: "no key" })]);
    expect(blocks).toEqual([{ kind: "error", message: "no key", at: "2026-09-25T12:00:00Z" }]);
  });
});

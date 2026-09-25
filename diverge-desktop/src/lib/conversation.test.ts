import { describe, expect, it } from "vitest";
import type { LogEntry } from "../bindings/LogEntry";
import type { LogItem } from "../bindings/LogItem";
import { fold } from "./conversation";

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
    expect(turn.parts[0]).toEqual({ kind: "reasoning", text: "Look first." });
    const tool = turn.parts[1];
    if (tool.kind !== "tool") throw new Error("expected a tool");
    expect(tool.answer).toEqual({ text: "1 broken", isError: false });
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

  it("shows an error where it happened", () => {
    const blocks = fold([e({ kind: "error", message: "no key" })]);
    expect(blocks).toEqual([{ kind: "error", message: "no key", at: "2026-09-25T12:00:00Z" }]);
  });
});

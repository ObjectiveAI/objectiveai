import { describe, expect, it } from "vitest";
import fixture from "../preview/fixture.json";
import { t } from "../strings";

describe("the agent door's tools", () => {
  it("each reads as words in a conversation, never as a tool's name", () => {
    const names = (fixture.door_tools as { name: string }[]).map((x) => x.name);
    expect(names.length).toBeGreaterThan(0);
    for (const name of names) expect(t.convo.doorTools[name], name).toBeTruthy();
  });
});

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

describe("a move's state", () => {
  // Every state the room program gives a move (diverge-desktop-room: first_state and derive).
  const states = ["shown", "open", "offered", "claimed", "delivered", "issued", "done", "asked", "left", "said", "taken", "declined", "closed"];
  it("each has words on screen, or is deliberately left unsaid", () => {
    for (const s of states) expect(t.spaces.states[s], s).toBeDefined();
  });
});

describe("the first-run page", () => {
  // The app runs on more than one kind of computer, and never reads the login.
  it("names no kind of computer, and says nothing came from a login", () => {
    for (const [key, words] of Object.entries(t.firstRun)) {
      expect(words, key).not.toMatch(/\b(mac|macos|windows|linux|login)\b/i);
    }
  });
});

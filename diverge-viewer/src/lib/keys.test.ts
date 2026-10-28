import { describe, expect, it } from "vitest";
import { escapeTop, opens, rove, type Escapable } from "./keys";

describe("the tab strip's arrow keys", () => {
  it("move one along, round the ends, and Home and End go to the ends", () => {
    expect(rove(4, 0, "ArrowRight")).toBe(1);
    expect(rove(4, 3, "ArrowRight")).toBe(0);
    expect(rove(4, 0, "ArrowLeft")).toBe(3);
    expect(rove(4, 2, "Home")).toBe(0);
    expect(rove(4, 1, "End")).toBe(3);
  });
  it("other keys move nowhere, and an empty strip has nowhere to go", () => {
    expect(rove(4, 1, "ArrowDown")).toBeNull();
    expect(rove(4, 1, "a")).toBeNull();
    expect(rove(0, 0, "ArrowRight")).toBeNull();
  });
  it("Enter and Space open a tab; nothing else does", () => {
    expect(opens("Enter")).toBe(true);
    expect(opens(" ")).toBe(true);
    expect(opens("ArrowRight")).toBe(false);
  });
});

describe("Escape", () => {
  const form = (shown: boolean, log: string[], name: string): Escapable => ({ shown: () => shown, close: () => log.push(name) });
  it("closes the newest inline form still on screen, and only that one", () => {
    const log: string[] = [];
    const open = [form(true, log, "rules"), form(true, log, "remove"), form(false, log, "in a hidden tab")];
    escapeTop(open)?.close();
    expect(log).toEqual(["remove"]);
  });
  it("closes nothing when no form is open on screen", () => {
    expect(escapeTop([])).toBeNull();
    expect(escapeTop([form(false, [], "hidden")])).toBeNull();
  });
});

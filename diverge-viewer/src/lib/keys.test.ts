import { describe, expect, it } from "vitest";
import { escapeTop, type Escapable } from "./keys";

describe("Escape", () => {
  const form = (shown: boolean, log: string[], name: string): Escapable => ({ shown: () => shown, close: () => log.push(name) });
  it("closes the newest inline form still on screen, and only that one", () => {
    const log: string[] = [];
    const open = [form(true, log, "rules"), form(true, log, "remove"), form(false, log, "in a hidden page")];
    escapeTop(open)?.close();
    expect(log).toEqual(["remove"]);
  });
  it("closes nothing when no form is open on screen", () => {
    expect(escapeTop([])).toBeNull();
    expect(escapeTop([form(false, [], "hidden")])).toBeNull();
  });
});

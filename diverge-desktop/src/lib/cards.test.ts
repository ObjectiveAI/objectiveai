import { describe, expect, it } from "vitest";
import type { CardView } from "../bindings/CardView";
import { afterCardEvent, noCards } from "./cards";
import { t } from "../strings";

const card = (id: number, agent = "research-notes"): CardView => ({
  id,
  agent,
  from: null,
  kind: "choice",
  question: "",
  options: ["yes", "no"],
  at: "2026-10-01T09:00:00Z",
  call: { room: "room-1", room_title: "Saturday Workshop", verb: "claim", reach: "work", arguments: { task_id: "task-1" }, about: "Package the photo resizer as a tool" },
  hire: null,
});

describe("a card's events", () => {
  it("a card waits once, however often it's heard", () => {
    let s = afterCardEvent(noCards, { event: "card", card: card(1) });
    s = afterCardEvent(s, { event: "card", card: card(1) });
    expect(s.cards.map((c) => c.id)).toEqual([1]);
  });

  it("an answered card is gone", () => {
    const s = afterCardEvent(afterCardEvent(noCards, { event: "card", card: card(1) }), { event: "answered", id: 1 });
    expect(s).toEqual(noCards);
  });

  it("a withdrawn card stops waiting and is kept, to say so", () => {
    let s = afterCardEvent(noCards, { event: "card", card: card(1) });
    s = afterCardEvent(s, { event: "card", card: card(2, "site-fixes") });
    s = afterCardEvent(s, { event: "withdrawn", id: 1 });
    expect(s.cards.map((c) => c.id)).toEqual([2]);
    expect(s.withdrawn.map((c) => c.id)).toEqual([1]);
    // Heard again, or answered late: nothing changes.
    expect(afterCardEvent(s, { event: "withdrawn", id: 1 })).toEqual(s);
    expect(afterCardEvent(s, { event: "answered", id: 1 })).toEqual(s);
  });

  it("a withdrawal for a card never seen changes nothing", () => {
    expect(afterCardEvent(noCards, { event: "withdrawn", id: 7 })).toEqual(noCards);
  });

  it("the screen has words for a withdrawn card", () => {
    expect(t.cards.withdrawn).toBeTruthy();
    expect(t.cards.withdrawnNote).toBeTruthy();
    expect(t.cards.stoppedWaiting).toBeTruthy();
  });
});

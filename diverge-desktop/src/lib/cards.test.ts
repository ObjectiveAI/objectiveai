import { describe, expect, it } from "vitest";
import type { CardView } from "../bindings/CardView";
import { afterAnswer, afterCardEvent, inTimeOrder, noCards } from "./cards";
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

  it("an answer the app took removes the card", () => {
    const s = afterAnswer(afterCardEvent(noCards, { event: "card", card: card(1) }), 1, true);
    expect(s).toEqual(noCards);
  });

  it("an answer the app refused leaves the card for its withdrawal to say so", () => {
    let s = afterCardEvent(noCards, { event: "card", card: card(1) });
    s = afterAnswer(s, 1, false);
    expect(s.cards.map((c) => c.id)).toEqual([1]);
    s = afterCardEvent(s, { event: "withdrawn", id: 1 });
    expect(s.withdrawn.map((c) => c.id)).toEqual([1]);
    expect(s.cards).toEqual([]);
  });

  it("a refusal after the withdrawal was heard changes nothing", () => {
    let s = afterCardEvent(noCards, { event: "card", card: card(1) });
    s = afterCardEvent(s, { event: "withdrawn", id: 1 });
    expect(afterAnswer(s, 1, false)).toEqual(s);
  });
});

describe("withdrawn cards among what happened", () => {
  const at = (id: number, when: string): CardView => ({ ...card(id), at: when });
  const items = [
    { at: "2026-10-01T09:00:00Z", n: "a" },
    { at: "2026-10-01T09:10:00Z", n: "b" },
    { at: "2026-10-01T09:20:00Z", n: "c" },
  ];

  it("each card sits where it happened, not at the end", () => {
    const placed = inTimeOrder(items, [at(2, "2026-10-01T09:15:00+00:00"), at(1, "2026-10-01T09:05:00+00:00")]);
    expect(placed.map((p) => ("card" in p ? `card ${p.card.id}` : p.item.n))).toEqual(["a", "card 1", "b", "card 2", "c"]);
  });

  it("a card later than everything comes last, and with no cards nothing moves", () => {
    const placed = inTimeOrder(items, [at(3, "2026-10-01T10:00:00Z")]);
    expect(placed.map((p) => ("card" in p ? `card ${p.card.id}` : p.item.n))).toEqual(["a", "b", "c", "card 3"]);
    expect(inTimeOrder(items, []).map((p) => ("item" in p ? p.index : -1))).toEqual([0, 1, 2]);
  });
});

import { describe, expect, it } from "vitest";
import type { MoveView } from "../bindings/MoveView";
import { cardVerbs, replyVerbs, withoutKnown, type Here } from "./moves";

const ME = "key-me";
const move = (over: Partial<MoveView>): MoveView => ({
  id: "m1", kind: "say", author: "someone", by: "key-other", member: null, erasable: false, agent_of: null, at: "2026-09-27T12:00:00Z",
  title: "", body: "", state: "", parent: null, fields: {}, charter: "1", hash: "h", ...over,
});
const room: Here = { you_key: ME, you_account: null, mine: false, tools: ["show", "ask", "offer", "take_offer", "close_ask", "reply", "withdraw", "erase"] };
const verbs = (xs: { verb: string }[]) => xs.map((x) => x.verb);

describe("a room's cards act on their own move", () => {
  it("your open ask closes from its card, with the ask already named", () => {
    const ask = move({ id: "ask-1", kind: "ask", by: ME, state: "open" });
    const close = cardVerbs(ask, room).find((v) => v.verb === "close_ask");
    expect(close?.prefill).toEqual({ ask_id: "ask-1" });
  });
  it("an ask that's taken, closed, or someone else's doesn't offer closing", () => {
    expect(verbs(cardVerbs(move({ kind: "ask", by: ME, state: "taken" }), room))).not.toContain("close_ask");
    expect(verbs(cardVerbs(move({ kind: "ask", by: ME, state: "closed" }), room))).not.toContain("close_ask");
    expect(verbs(cardVerbs(move({ kind: "ask", state: "open" }), room))).toEqual(["reply", "offer"]);
  });
  it("an offer on your open ask is taken from the offer itself", () => {
    const ask = move({ id: "ask-1", kind: "ask", by: ME, state: "open" });
    const offer = move({ id: "offer-3", kind: "offer", parent: "ask-1" });
    expect(replyVerbs(offer, ask, room)).toEqual([expect.objectContaining({ verb: "take_offer", prefill: { offer_id: "offer-3" } })]);
  });
  it("no taking an offer on someone else's ask, on an ask no longer open, or in a room without the verb", () => {
    const offer = move({ id: "offer-3", kind: "offer", parent: "ask-1" });
    expect(replyVerbs(offer, move({ kind: "ask", state: "open" }), room)).toEqual([]);
    expect(replyVerbs(offer, move({ kind: "ask", by: ME, state: "taken" }), room)).toEqual([]);
    expect(replyVerbs(offer, move({ kind: "ask", by: ME, state: "open" }), { ...room, tools: ["reply"] })).toEqual([]);
  });
  it("under rules 2 your ask is yours by your account, from any of your devices", () => {
    const here = { ...room, you_account: "acct-me" };
    const ask = move({ kind: "ask", by: "key-other-device", member: "acct-me", state: "open" });
    expect(verbs(cardVerbs(ask, here))).toContain("close_ask");
  });
  it("taking back is yours; erasing someone else's words is the host's", () => {
    const said = move({ kind: "say", erasable: true });
    expect(verbs(cardVerbs({ ...said, by: ME }, room))).toContain("withdraw");
    expect(verbs(cardVerbs(said, room))).not.toContain("erase");
    expect(verbs(cardVerbs(said, { ...room, mine: true }))).toContain("erase");
  });
});

describe("a verb's form on a card", () => {
  it("leaves out what the card already knows, and asks only for the rest", () => {
    const schema = { type: "object", properties: { ask_id: { type: "string" }, note: { type: "string" } }, required: ["ask_id"] };
    expect(withoutKnown(schema, { ask_id: "ask-1" })).toEqual({ type: "object", properties: { note: { type: "string" } }, required: [] });
  });
});

import type { MoveView } from "../bindings/MoveView";
import { t } from "../strings";

/** One verb a card offers: the room's verb, what the card already knows to send, and its words. */
export type CardVerb = { verb: string; label: string; prefill: Record<string, unknown> };

/** What a card needs to know about the room it's in: who you are there, whether you host it, and its verbs. */
export type Here = { you_key: string; you_account: string | null; mine: boolean; tools: string[] };

/** Whether a member's key is you here: the key you seal with, or, in a room under rules 2, the account it's on. */
export const isYou = (h: Pick<Here, "you_key" | "you_account">, key?: string | null) => !!key && (key === h.you_key || key === h.you_account);

/** Under rules 2 a move names its maker's account, so it's yours from any of your devices. */
export const madeByYou = (h: Here, m: MoveView) => isYou(h, m.member ?? m.by);

/** The verbs a move's own card offers, in order: each acts on that move, so its form never asks which one. */
export function cardVerbs(m: MoveView, h: Here): CardVerb[] {
  const has = (name: string) => h.tools.includes(name);
  const f = m.fields as Record<string, unknown>;
  const isMe = madeByYou(h, m);
  const claimedByMe = isYou(h, f.claimed_by_key as string | undefined);
  const out: CardVerb[] = [];
  const add = (verb: string, label: string, prefill: Record<string, unknown>) => out.push({ verb, label, prefill });
  if (has("reply") && m.kind !== "reply" && f.sealed !== true) add("reply", t.spaces.reply, { move_id: m.id });
  if (m.kind === "ask" && !isMe && has("offer")) add("offer", t.spaces.moveKinds.offer, { ask_id: m.id });
  if (m.kind === "ask" && isMe && m.state === "open" && has("close_ask")) add("close_ask", t.home.gotIt, { ask_id: m.id });
  if (m.kind === "task" && m.state === "open" && !isMe && has("claim")) add("claim", t.spaces.claim, { task_id: m.id });
  if (m.kind === "task" && m.state === "claimed" && claimedByMe && has("deliver")) add("deliver", t.spaces.deliver, { task_id: m.id });
  if (m.kind === "task" && m.state === "delivered" && isMe && has("accept")) add("accept", t.spaces.accept, { task_id: m.id });
  const canSettle = m.kind === "task" && typeof f.pledge === "string" && (m.state === "delivered" || m.state === "done") && (isMe || claimedByMe) && has("settle");
  if (canSettle) {
    add("settle", t.spaces.settledYes, { task_id: m.id, agree: true });
    add("settle", t.spaces.settledNo, { task_id: m.id, agree: false });
  }
  if (m.kind === "direction" && has("steer")) {
    add("steer", t.spaces.prefer, { direction_id: m.id, move: "prefer" });
    add("steer", t.spaces.reject, { direction_id: m.id, move: "reject" });
    add("steer", t.spaces.noteVerb, { direction_id: m.id, move: "note" });
  }
  return out.concat(eraseVerbs(m, h));
}

/** The verbs a reply under a card offers: taking an offer on your own open ask, and taking back or erasing words. */
export function replyVerbs(r: MoveView, parent: MoveView, h: Here): CardVerb[] {
  const out: CardVerb[] = [];
  if (r.kind === "offer" && parent.kind === "ask" && parent.state === "open" && madeByYou(h, parent) && !madeByYou(h, r) && h.tools.includes("take_offer")) {
    out.push({ verb: "take_offer", label: t.home.takeThis, prefill: { offer_id: r.id } });
  }
  return out.concat(eraseVerbs(r, h));
}

/** Taking back your own words, or, for the host, erasing someone else's with a reason. The room says which moves
 *  it would erase now. */
function eraseVerbs(x: MoveView, h: Here): CardVerb[] {
  if (!x.erasable) return [];
  if (madeByYou(h, x) && h.tools.includes("withdraw")) return [{ verb: "withdraw", label: t.spaces.takeBack, prefill: { move_id: x.id } }];
  if (!madeByYou(h, x) && h.mine && h.tools.includes("erase")) return [{ verb: "erase", label: t.spaces.erase, prefill: { move_id: x.id } }];
  return [];
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Schema = Record<string, any>;

/** A verb's form on a card: the fields the card already filled are left out, so it never asks for a move's id. */
export function withoutKnown(schema: Schema, prefill: Record<string, unknown>): Schema {
  const known = new Set(Object.keys(prefill));
  const properties = Object.fromEntries(Object.entries((schema.properties ?? {}) as Schema).filter(([k]) => !known.has(k)));
  const required = Array.isArray(schema.required) ? schema.required.filter((k: string) => !known.has(k)) : schema.required;
  return { ...schema, properties, ...(required ? { required } : {}) };
}

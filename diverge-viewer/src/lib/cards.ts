import type { CardEvent } from "../bindings/CardEvent";
import type { CardView } from "../bindings/CardView";
import type { KnockView } from "../bindings/KnockView";
import { t } from "../strings";

/** The cards waiting on you, and those withdrawn because the agent stopped waiting: kept to say so. */
export type Cards = { cards: CardView[]; withdrawn: CardView[] };

export const noCards: Cards = { cards: [], withdrawn: [] };

/** The cards after one event from the app. A withdrawn card stops waiting; nothing answers it any more. */
export function afterCardEvent(s: Cards, e: CardEvent): Cards {
  switch (e.event) {
    case "card":
      return s.cards.some((c) => c.id === e.card.id) ? s : { ...s, cards: [...s.cards, e.card] };
    case "answered":
      return s.cards.some((c) => c.id === e.id) ? { ...s, cards: s.cards.filter((c) => c.id !== e.id) } : s;
    case "withdrawn": {
      const card = s.cards.find((c) => c.id === e.id);
      return card ? { cards: s.cards.filter((c) => c.id !== e.id), withdrawn: [...s.withdrawn, card] } : s;
    }
    case "end":
      return s;
  }
}

/** The cards after the app answers the person's answer to one: taken, it's gone; refused, the card was
 *  answered or withdrawn meanwhile, and stays until the app's own event says which. */
export function afterAnswer(s: Cards, id: number, taken: boolean): Cards {
  return taken ? afterCardEvent(s, { event: "answered", id }) : s;
}

/** Log items in their order, with each card placed after the last item that came no later than it. */
export function inTimeOrder<T extends { at: string }>(items: T[], cards: CardView[]): ({ item: T; index: number } | { card: CardView })[] {
  const when = (at: string) => {
    const ms = Date.parse(at);
    return Number.isNaN(ms) ? 0 : ms;
  };
  const waiting = [...cards].sort((x, y) => when(x.at) - when(y.at));
  const out: ({ item: T; index: number } | { card: CardView })[] = [];
  items.forEach((item, index) => {
    while (waiting.length > 0 && when(waiting[0].at) < when(item.at)) out.push({ card: waiting.shift()! });
    out.push({ item, index });
  });
  for (const card of waiting) out.push({ card });
  return out;
}

/** A card in one line, in the screen's words: what the agent wants to do, or what a visitor asks. */
export function cardLine(c: CardView): string {
  if (c.hire) {
    const pledge = c.hire.pledge ? ` ${t.cards.hire.pledge} ${c.hire.pledge}.` : "";
    return `${t.cards.hire.someone} ${c.hire.from}${c.hire.mark ? ` (${c.hire.mark})` : ""} ${t.cards.hire.asks} ${c.agent}${t.cards.hire.through} “${c.hire.what}”.${pledge}`;
  }
  if (c.call) {
    const verb = t.cards.verbs[c.call.verb] ?? c.call.verb.replace(/_/g, " ");
    const about = c.call.about ? ` “${c.call.about}”` : "";
    const where = c.call.room ? ` ${t.cards.inRoom} ${c.call.room_title}` : `: ${c.call.room_title}`;
    return `${t.cards.wantsTo} ${verb}${about}${where}`;
  }
  return c.question;
}

/** A card's fine print: for a hire, how it runs; for a move, what kind of move it is. */
export function cardNote(c: CardView): string | null {
  if (c.hire) return `${t.cards.hire.runs} ${c.agent} ${t.cards.hire.busy}`;
  if (c.call) return t.cards.reachNote[c.call.reach];
  return null;
}

/** Everything the call would send, as label and value: nothing the card leaves out. */
export function cardDetails(c: CardView): [string, string][] {
  if (!c.call) return [];
  const skip = new Set(["task_id", "ask_id", "move_id", "direction_id", "hire_id", "space"]);
  return Object.entries(c.call.arguments)
    .filter(([k, v]) => !skip.has(k) && v !== null && v !== undefined && v !== "")
    .map(([k, v]) => [t.cards.args[k] ?? k.replace(/_/g, " "), Array.isArray(v) ? v.join(", ") : typeof v === "boolean" ? (v ? t.cards.answers.yes : t.cards.answers.no) : typeof v === "object" ? JSON.stringify(v) : String(v)]);
}

/** A card's answer, in the screen's words. What goes back is the answer itself. */
export function answerLabel(o: string): string {
  return t.cards.answers[o] ?? o;
}

/** What a screen reader says as cards and knocks arrive: one line for each not seen before, in the screen's words. */
export function arrivals(seen: Set<string>, cards: CardView[], knocks: KnockView[]): string[] {
  const out: string[] = [];
  for (const c of cards) if (!seen.has(`card:${c.id}`)) out.push(`${c.agent} ${t.cards.asksYou}: ${cardLine(c)}`);
  for (const k of knocks) if (!seen.has(`knock:${k.knock_id}`)) out.push(`${k.name} ${t.rail.atTheDoor}: ${k.space_title}`);
  return out;
}

/** The keys `arrivals` remembers each card and knock by. */
export const arrivalKeys = (cards: CardView[], knocks: KnockView[]) => [...cards.map((c) => `card:${c.id}`), ...knocks.map((k) => `knock:${k.knock_id}`)];

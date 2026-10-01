import type { CardEvent } from "../bindings/CardEvent";
import type { CardView } from "../bindings/CardView";
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

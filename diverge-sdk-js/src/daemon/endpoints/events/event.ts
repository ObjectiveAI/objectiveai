// One edge of the graph: who sent, who received, how often.

import { z } from "zod";
import type { Keyed } from "../../../shared/listing/keyed.js";
import { Party } from "./party.js";

/**
 * One event, as a list reports it: the sender, the receiver, and how
 * many times something has passed between them. One per sender and
 * receiver; the id is the daemon's, given once
 * when the edge is added, so the latest edges are the highest.
 */
export const Event = z.object({
  /** The daemon's number for this edge, given when it was added. */
  id: z.number().int().nonnegative(),
  /** Who sent: one `Party`. */
  sender: Party,
  /** Who received: one `Party`. */
  receiver: Party,
  /** How many times it has passed: `1` when added, one more at every occurrence since. */
  count: z.number().int().nonnegative(),
});
export type Event = z.infer<typeof Event>;

/** Keyed by the id: a `changed` event is the whole record with its count raised, a `removed` one the record as last sent. */
export const key: Keyed<Event> = (event) => event.id;

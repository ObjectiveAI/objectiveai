// One edge of the graph: who sent, who received, what, how often.

import { z } from "zod";
import { Creator } from "../../creator/creator.js";
import type { Keyed } from "../../../shared/listing/keyed.js";
import { Action } from "./action.js";

/**
 * One event, as a list reports it: the sender, the receiver, the
 * action between them, and how many times it has passed. One per
 * sender, receiver and action; the id is the daemon's, given once
 * when the edge is added, so the latest edges are the highest.
 */
export const Event = z.object({
  /** The daemon's number for this edge, given when it was added. */
  id: z.number().int().nonnegative(),
  /** Who sent. */
  sender: Creator,
  /** Who received. */
  receiver: Creator,
  /** What passed. */
  action: Action,
  /** How many times it has passed: `1` when added, one more at every occurrence since. */
  count: z.number().int().nonnegative(),
});
export type Event = z.infer<typeof Event>;

/** Keyed by the id: a `changed` event is the whole record with its count raised. */
export const key: Keyed<Event> = (event) => event.id;

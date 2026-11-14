// One grant's worth over images.

import { z } from "zod";
import { within } from "../../../shared/permission/within.js";
import { Action } from "../../../shared/permission/images/action.js";
import { Reach } from "../providers/reach.js";

/**
 * What one grant over images allows: an object of `Action`s and how
 * far they reach — `"any"`, or a `Reach`, the providers whose Stores
 * of the daemon's they act on, by identity, by their records' tags,
 * or both. An object that is not this does not decode, and neither
 * does a bare array of actions.
 */
export const Permission = z.strictObject({
  /** The actions. */
  actions: z.array(Action),
  /** How far: `"any"`, or the reach as a test. */
  within: within(Reach),
});
export type Permission = z.infer<typeof Permission>;

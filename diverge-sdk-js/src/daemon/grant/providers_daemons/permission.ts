// One grant's worth through providers.

import { z } from "zod";
import { within } from "../../../shared/permission/within.js";
import { Over } from "../../../shared/permission/providers_daemons/over.js";
import { Reach } from "../providers/reach.js";

/**
 * What one grant through providers allows: an object of `Over`
 * actions and how far they reach — `"any"`, or a `Reach`, the
 * providers by identity, by their records' tags, or both. An object
 * that is not this does not decode.
 */
export const Permission = z.strictObject({
  /** The actions. */
  actions: z.array(Over),
  /** How far: `"any"`, or the reach as a test. */
  within: within(Reach),
});
export type Permission = z.infer<typeof Permission>;

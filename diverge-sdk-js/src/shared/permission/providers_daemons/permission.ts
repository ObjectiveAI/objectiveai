// One grant's worth through providers, naming none of them.

import { z } from "zod";
import { Tags } from "../tags.js";
import { within } from "../within.js";
import { Over } from "./over.js";

/**
 * What one grant through providers allows, in the one shape
 * `permission` states for actions over what exists: an object of
 * `Over` actions and how far they reach, by the providers' tags. An
 * object that is not this does not decode.
 */
export const Permission = z.strictObject({
  /** The actions. */
  actions: z.array(Over),
  /** How far: `"any"`, or the tags as a test. */
  within: within(Tags),
});
export type Permission = z.infer<typeof Permission>;

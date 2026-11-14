// One grant's worth over providers.

import { z } from "zod";
import { Tagging } from "../../../shared/permission/tagging.js";
import { within } from "../../../shared/permission/within.js";
import { Make } from "../../../shared/permission/providers/make.js";
import { Over } from "../../../shared/permission/providers/over.js";
import { Reach } from "./reach.js";

/**
 * What one grant over providers allows, in one of three shapes told
 * apart on the wire by their very form: a bare array of `Make`
 * actions; an object of `Over` actions and how far they reach —
 * `"any"`, or a `Reach`, by identity, by tags, or both; or an object
 * of tagging actions, how far they reach, and which tags. The shapes
 * are tried in this order, as serde tries them.
 */
export const Permission = z.union([
  /** To make: the `Make` actions held, as a bare array. Nothing is judged but that the action is held. */
  z.array(Make),
  z.strictObject({
    /** The actions. */
    actions: z.array(Over),
    /** How far: `"any"`, or the reach as a test. */
    within: within(Reach),
  }),
  z.strictObject({
    /** `tag`, `untag`, or both. */
    actions: z.array(Tagging),
    /** How far: `"any"`, or the reach as a test. */
    within: within(Reach),
    /** Which tags: `"any"`, or only these. */
    tags: within(z.array(z.string())),
  }),
]);
export type Permission = z.infer<typeof Permission>;

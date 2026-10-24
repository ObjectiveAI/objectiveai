// One grant's worth over accounts.

import { z } from "zod";
import { Tagging } from "../../../shared/permission/tagging.js";
import { within } from "../../../shared/permission/within.js";
import { Make } from "../../../shared/permission/accounts/make.js";
import { Over } from "../../../shared/permission/accounts/over.js";
import { Filter } from "../../endpoints/accounts/list/client/request/filter.js";

/**
 * What one grant over accounts allows, in one of three shapes told
 * apart on the wire by their very form: a bare array of `Make` actions; an
 * object of `Over` actions and how far they reach — `"any"`, or the
 * family's list `Filter` read as a test; or an object of tagging
 * actions, how far they reach, and which tags. The shapes are tried
 * in this order, as serde tries them.
 */
export const Permission = z.union([
  /** To make: the `Make` actions held, as a bare array. Nothing is judged but that the action is held. */
  z.array(Make),
  z.strictObject({
    /** The actions. */
    actions: z.array(Over),
    /** How far: `"any"`, or the filter as a test. */
    within: within(Filter),
  }),
  z.strictObject({
    /** `tag`, `untag`, or both. */
    actions: z.array(Tagging),
    /** How far: `"any"`, or the filter as a test. */
    within: within(Filter),
    /** Which tags: `"any"`, or only these. */
    tags: within(z.array(z.string())),
  }),
]);
export type Permission = z.infer<typeof Permission>;

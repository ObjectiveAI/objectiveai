// The one test a grant that names nothing reaches by.

import { z } from "zod";

/**
 * Which of a kind a grant reaches, by their tags: those carrying
 * every one of `all_tags` and any one of `any_tags`. A member that
 * is empty is absent, and passes everything; both empty is every one
 * of the kind, which `"any"` says more plainly. Read as a TEST: a
 * thing passes when it passes every member given. No other member
 * decodes: a `within` that named something would otherwise read as
 * no tags at all, which is everything, and a grant must never widen
 * by being misread.
 */
export const Tags = z.strictObject({
  /** Every one of these among the thing's tags. Absent when empty, and then any tags. */
  all_tags: z.array(z.string()).optional(),
  /** Any one of these among the thing's tags; with `all_tags`, both hold. Absent when empty. */
  any_tags: z.array(z.string()).optional(),
});
export type Tags = z.infer<typeof Tags>;

// Every one, or only these.

import { z } from "zod";

/**
 * How far a grant reaches, or which tags it may touch: every one
 * there is, or only what `only` names. Flat on the wire: the string
 * `"any"`, or the value as it is — a filter object, a list of tags.
 * `"any"` is no filter and no list, so the two shapes never collide;
 * a string that is not `"any"` does not decode. The generic of the
 * Rust, as a function of the schema it is over.
 */
export function within<T extends z.ZodType>(only: T) {
  return z.union([z.literal("any"), only]);
}
export type Within<T> = "any" | T;

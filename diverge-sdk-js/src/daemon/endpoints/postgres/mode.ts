// Which database the daemon serves: its own, or one it dials.

import { z } from "zod";

/**
 * Which database the daemon splices container connections onto, and
 * keeps its own records in: its configuration, as a get answers it.
 * One object naming its kind: `{"kind":"local"}`, or
 * `{"kind":"remote","url":…}`, the URL with its password taken out.
 */
export const Mode = z.discriminatedUnion("kind", [
  /** A Postgres the daemon runs itself, beside it. */
  z.object({ kind: z.literal("local") }),
  /** A Postgres the daemon dials, wherever the URL says. */
  z.object({
    kind: z.literal("remote"),
    /** Where to dial and as whom, as a Postgres URL; a password in it is kept and never answered. */
    url: z.string(),
  }),
]);
export type Mode = z.infer<typeof Mode>;

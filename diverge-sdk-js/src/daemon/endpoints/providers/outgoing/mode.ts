// How the daemon authenticates to a provider it dials.

import { z } from "zod";

/**
 * The mode the daemon dials a provider in, and what that mode needs.
 * One object whose one member is the mode, by name:
 * `{"unbrokered":{"authorization":…}}`. One mode today; a brokered
 * mode is a second member when the wire defines it.
 */
export const Mode = z.union([
  z.strictObject({
    /** The two ends already know each other: the daemon presents a credential the provider was told to expect. */
    unbrokered: z.object({
      /** The credential, as the daemon presents it: a bearer token, an API key, whatever the provider agreed to. Given on an add or an edit, and answered by nothing. */
      authorization: z.string(),
    }),
  }),
]);
export type Mode = z.infer<typeof Mode>;

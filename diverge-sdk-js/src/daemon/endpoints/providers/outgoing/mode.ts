// How the daemon authenticates to a provider it dials.

import { z } from "zod";

/**
 * The mode the daemon dials a provider in, and what that mode needs.
 * One object whose one member is the mode, by name:
 * `{"unbrokered":{"key":…}}`. One mode today; a brokered
 * mode is a second member when the wire defines it.
 */
export const Mode = z.union([
  z.strictObject({
    /** The two ends already know each other: the daemon presents a credential the provider was told to expect. */
    unbrokered: z.object({
      /** The name of the entry in the daemon's host's keyring that holds the key the daemon presents. A name, never the key: no request carries a key. */
      key: z.string(),
    }),
  }),
]);
export type Mode = z.infer<typeof Mode>;

// One account this daemon holds on another daemon.

import { z } from "zod";
import { Kind } from "../outgoing/kind.js";

/**
 * One account on a daemon record: its name, of the caller's
 * choosing, and the kind of its mode without the credential, which
 * the wire never answers. What a connected tool and a relayed
 * request name the daemon by, beside the record's name.
 */
export const Account = z.object({
  /** The name, as its add gave it: unique among the record's accounts. */
  name: z.string(),
  /** Which mode this daemon authenticates to it in, without the credential. */
  kind: Kind,
});
export type Account = z.infer<typeof Account>;

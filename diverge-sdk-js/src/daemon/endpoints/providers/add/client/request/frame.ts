// A providers add request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Direction } from "../../../direction.js";
import { Description } from "../../../../../../shared/description.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";

/**
 * Ask the daemon to keep a provider's record, and to work with it from
 * now on: dial it when it is outgoing, admit it when it is incoming.
 * What the daemon holds open on the provider while it is connected is
 * three members' word, each independent of the other two:
 *
 * 1. `announcement`. Present, the daemon announces itself on the
 *    provider with that text — it holds the provider's
 *    `daemons::announce` open, and is in its `daemons::list`. Absent,
 *    it does not announce itself there.
 * 2. `accept_daemons`. `true`, the daemon accepts connections from
 *    other daemons through the provider — it holds the provider's
 *    `daemons::accept` open. Absent, `true`.
 * 3. `messageable`. `true`, the daemon listens for messages on the
 *    provider — it holds the provider's `daemons::listen` open.
 *    Absent, `false`.
 */
export const Frame = z.intersection(
  z.object({
    /** The provider, by the identity its token establishes. */
    identity: Identity,
    /** What the daemon announces itself with on the provider: rule 1. Absent, the daemon does not announce itself there. */
    announcement: Description.optional(),
    /** Whether the daemon accepts connections from other daemons through the provider: rule 2. Absent, `true`. */
    accept_daemons: z.boolean().optional(),
    /** Whether the daemon listens for messages on the provider: rule 3. Absent, `false`. */
    messageable: z.boolean().optional(),
    /** Whether the daemon works with it from now on. Absent, `true`. */
    enabled: z.boolean().optional(),
    /** What it is, in the caller's words: the operator's note, which the provider never sees. Absent, none. */
    description: z.string().optional(),
    /** The tags to put on it. Absent, none. */
    tags: z.array(z.string()).optional(),
    /** Served by another daemon: see `Relay`. Absent, served here. */
    relay: Relay.optional(),
  }),
  Direction,
);
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 136;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}

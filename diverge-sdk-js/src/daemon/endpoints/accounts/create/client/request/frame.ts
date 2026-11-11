// Creating an account: an identity, with what it is for and the roles it holds.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";

/**
 * Ask the daemon to create an account: an identity it admits from
 * now on, with `description` and `roles` beside. An identity that is
 * an account's already is refused; so is a role the daemon does not
 * have. Creating takes the `create` grant over accounts and the
 * `grant` grant over every role named.
 */
export const Frame = z.object({
  /** The identity the account is: the `(issuer, subject)` a client's token must establish to be served as it. */
  identity: Identity,
  /** What the account is for, in words of the caller's; compared by nobody and read by people. Absent, none. */
  description: z.string().optional(),
  /** The roles it holds, by name, each one the daemon has and the caller holds the `grant` grant over. Empty, an account that may do nothing. */
  roles: z.array(z.string()).optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 45;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}

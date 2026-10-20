// Creating an account: a name, a credential without its key, or both, with what it is for and the roles it holds.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Named, Unnamed } from "../../../../../../daemon/endpoints/accounts/definition.js";

/**
 * Ask the daemon to create an account. The definition is flattened
 * in, so the request is `{"name":…}`, `{"credential":…}` or both,
 * with `description` and `roles` beside; a credential carries no
 * key, which the daemon mints and answers. A name or an identity
 * that is an account's already is refused; so is a role the daemon
 * does not have. Creating takes the `create` grant over accounts
 * and the `grant` grant over every role named.
 */
export const Frame = z.union([
  /** An account with a name, and a credential if any client may dial in as it. */
  Named.extend({
  /** What the account is for, in words of the caller's; compared by nobody and read by people. Absent, none. */
  description: z.string().optional(),
  /** The roles it holds, by name, each one the daemon has and the caller holds the `grant` grant over. Empty, an account that may do nothing. */
  roles: z.array(z.string()).optional(),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
  }),
  /** An account without a name: a client dials in as it, and no container runs under it. */
  Unnamed.extend({
  /** What the account is for, in words of the caller's; compared by nobody and read by people. Absent, none. */
  description: z.string().optional(),
  /** The roles it holds, by name, each one the daemon has and the caller holds the `grant` grant over. Empty, an account that may do nothing. */
  roles: z.array(z.string()).optional(),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
  }),
]);
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 45;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}

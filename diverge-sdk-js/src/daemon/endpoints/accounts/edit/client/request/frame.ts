// Changing an account: a patch to its name, its credential, its description, its roles.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Reference } from "../../../../../../daemon/endpoints/accounts/reference.js";
import { Patch } from "../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change an account: a patch to its editable
 * members. The patch is applied to the account's `Editable`
 * document, whole or not at all, and the result is the account from
 * then on. A patch that writes at `/credential` or beneath it
 * replaces the credential: the daemon mints a new key when the
 * result has one, answered once — how a key rotates. A result with
 * neither a name nor a credential is refused as leaving the account
 * alone.
 */
export const Frame = z.object({
  /** The account to change. */
  account: Reference,
  /** The patch, applied to the `Editable` document. Empty, nothing changes. */
  patch: Patch,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 49;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}

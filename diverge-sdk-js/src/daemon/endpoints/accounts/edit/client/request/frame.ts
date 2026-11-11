// Changing an account: a patch to its description and its roles.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Reference } from "../../../../../../daemon/endpoints/accounts/reference.js";
import { Patch } from "../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change an account: a patch to its editable
 * members. The patch is applied to the account's `Editable`
 * document, whole or not at all, and the result is the account from
 * then on. The identity is not editable: another identity is another
 * account.
 */
export const Frame = z.object({
  /** The account to change. */
  account: Reference,
  /** The patch, applied to the `Editable` document. Empty, nothing changes. */
  patch: Patch,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 49;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}

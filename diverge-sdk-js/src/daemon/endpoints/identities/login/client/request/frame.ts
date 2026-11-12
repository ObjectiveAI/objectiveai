// Signing an identity in on the daemon's host.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";

/**
 * Ask the daemon to sign an identity in on its host, as `diverge
 * identities login` signs one in on the host it runs on: by the
 * device authorization grant.
 */
export const Frame = z.object({
  /** The issuer, exactly as its discovery document names it. */
  issuer: z.string(),
  /** The subject to sign in as: the `sub` the approval must name. */
  subject: z.string(),
  /** The public client the daemon signs in as at the issuer. */
  client_id: z.string(),
  /** The scope asked for, space-separated; `openid` and `offline_access` are added where it lacks them. Absent, those two alone. */
  scope: z.string().optional(),
  /** What the access token is for, RFC 8707's `resource`. Absent, the issuer's own choice. */
  resource: z.string().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 128;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}

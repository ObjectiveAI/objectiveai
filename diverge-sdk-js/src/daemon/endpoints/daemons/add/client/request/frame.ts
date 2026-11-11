// A daemons add request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Routes } from "../../../routes.js";

/** Ask the daemon to record a remote daemon, the account of ours it is reached as, and the routes it may be reached by. Our identity's issuer must be the remote's. The record's ID is the hash of those, and the answer carries it. */
export const Frame = z.object({
  ...Routes.shape,
  /** What it is, in the caller's words. Absent, none. */
  description: z.string().optional(),
  /** The remote daemon, by the identity it answers a handshake as. */
  daemon: Identity,
  /** The identity this daemon presents to it, one of the host's, at the remote's issuer. */
  identity: Identity,
  /** The tags to put on it. Absent, none. */
  tags: z.array(z.string()).optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;


/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 130;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}

// A providers edit request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Ip } from "../../../ip.js";
import { Description } from "../../../../../../shared/description.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";

/**
 * Ask the daemon to change a provider's record, by its identity. The
 * rules:
 *
 * 1. Given replaces, absent leaves. Every member given replaces what
 *    the record holds, and every member absent leaves it.
 * 2. Three ways for two. `announcement` and `ips` are each named three
 *    ways: absent leaves the record's, `null` clears it, and a value
 *    replaces it. A cleared `announcement` is a daemon that does not
 *    announce itself on the provider; cleared `ips` admit any IP.
 * 3. Not editable. The record's identity names it and does not change.
 *    Its direction — incoming or outgoing — and an outgoing record's
 *    `address` do not change either: a provider reached another way is
 *    a delete and an add. Whether it is enabled is changed only by
 *    `enable` and `disable`, and its tags by `tag` and `untag`.
 * 4. Unknown members. A request naming any member but these —
 *    `direction` and `address` among them — does not parse.
 * 5. `ips` is an incoming record's. `ips` given for an outgoing record,
 *    as a value or as `null`, is refused with an error, and nothing
 *    changes.
 * 6. The live connection. Nothing an edit changes closes the provider's
 *    connection, except `ips` that no longer admit the live
 *    connection's source address: that connection is closed then. Every
 *    other change is made on the connection as it stands: the daemon
 *    cancels and opens the provider's `daemons::announce`,
 *    `daemons::accept` and `daemons::listen` to match the record.
 */
export const Frame = z.strictObject({
  /** The provider, by the identity its token establishes. */
  identity: Identity,
  /** What the daemon announces itself with on the provider, in place of the record's: rule 2. `null`, the daemon stops announcing itself there. */
  announcement: Description.nullable().optional(),
  /** The source IPs an incoming provider may dial from, in place of the record's: rules 2 and 5. `null`, any IP. */
  ips: z.array(Ip).nullable().optional(),
  /** Whether the daemon accepts connections from other daemons through the provider, in place of the record's. */
  accept_daemons: z.boolean().optional(),
  /** Whether the daemon listens for messages on the provider, in place of the record's. */
  messageable: z.boolean().optional(),
  /** The description — the operator's note — in place of the record's. */
  description: z.string().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 137;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}

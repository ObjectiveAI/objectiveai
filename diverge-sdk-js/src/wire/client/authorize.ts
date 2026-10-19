// The step in front: presenting the credential.

import type { Connection } from "../connection.js";
import { unbrokered } from "../frame/auth.js";
import { encodeClientFrame } from "../frame/client.js";

/**
 * This end dialled, so this end authenticates: the credential goes
 * out as the first frame and nothing is read or waited for. There
 * is no answer to wait FOR — an accepted credential is followed by
 * the connection simply working, a rejected one by a close, which a
 * caller sees as its first scope closing.
 */
export function authorize(connection: Connection, key: string): void {
  connection.send(encodeClientFrame({ kind: "auth", payload: unbrokered(key) }));
}

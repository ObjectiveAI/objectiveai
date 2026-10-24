// The step in front: the handshake, as the connector.

import type { Connection } from "../connection.js";
import { auth, readAuth } from "../frame/auth.js";
import { encodeClientFrame } from "../frame/client.js";
import { decodeServerFrame } from "../frame/server.js";
import type { Authenticator, Peer } from "./authenticator.js";
import { HandshakeError } from "./handshake_error.js";

/**
 * The handshake from the end that dialled:
 *
 * 1. This end's token from `issuer` goes out as the first frame, and
 *    nothing else is sent until the answer has been checked.
 * 2. The daemon's first frame must be its own token: an auth frame,
 *    well formed.
 * 3. That token is verified, and its issuer must be `issuer`, exactly.
 *
 * Every failure closes the socket and throws the `HandshakeError`;
 * nothing is sent after it, since a failed handshake is a close and
 * nothing else in either direction. The router starts only once this
 * resolves, so the one lawful auth frame never reaches it.
 */
export async function authorize(connection: Connection, issuer: string, authenticator: Authenticator): Promise<Peer> {
  try {
    const token = await authenticator.present(issuer).catch((cause: unknown) => {
      throw new HandshakeError("present", cause);
    });
    connection.send(encodeClientFrame({ kind: "auth", payload: auth(token) }));
    const bytes = await connection.frames.next();
    if (bytes === undefined) {
      throw new HandshakeError("ended");
    }
    let answer: string;
    try {
      const frame = decodeServerFrame(bytes);
      if (frame.kind !== "auth") {
        throw new HandshakeError("not_auth");
      }
      answer = readAuth(frame.payload);
    } catch (cause) {
      throw cause instanceof HandshakeError ? cause : new HandshakeError("malformed", cause);
    }
    const peer = await authenticator.verify(answer).catch((cause: unknown) => {
      throw new HandshakeError("refused", cause);
    });
    if (peer.issuer !== issuer) {
      throw new HandshakeError("issuer");
    }
    return peer;
  } catch (cause) {
    connection.close();
    throw cause instanceof HandshakeError ? cause : new HandshakeError("send", cause);
  }
}

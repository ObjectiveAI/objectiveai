// The step in front: the handshake, as the connector.

import type { Connection } from "../connection.js";
import { offer, proof, readAnswer } from "../frame/auth.js";
import type { Answer } from "../frame/auth.js";
import { encodeClientFrame } from "../frame/client.js";
import { decodeServerFrame } from "../frame/server.js";
import { owned } from "../seal.js";
import type { Authenticator, Peer } from "./authenticator.js";
import { HandshakeError } from "./handshake_error.js";
import { proven, seals, signed, thumbprint, transcript } from "./transcript.js";

/** The curve of every key in the handshake. */
const CURVE = { name: "ECDH", namedCurve: "P-256" } as const;

/**
 * The handshake from the end that dialled:
 *
 * 1. This end's token, its key and its signer, from `authenticator`,
 *    and a one-time P-256 key pair made for this connection. The
 *    pair's public half, the identity key and the token go out as the
 *    first frame.
 * 2. The daemon's frame must be an auth frame, well formed: its
 *    one-time key, its identity key, its signature, its token.
 * 3. That token is verified by `authenticator`, which answers the
 *    thumbprint it is bound to. Its issuer need not be this end's.
 * 4. The daemon's identity key is the one its token names: its
 *    thumbprint is the token's, or the handshake ends.
 * 5. The daemon's signature is checked against that key, over the
 *    transcript: the daemon holds the key, or the handshake ends.
 * 6. The shared secret is agreed, and the two session keys drawn. A
 *    one-time key that is no point of the curve ends the handshake.
 * 7. This end's signature over the transcript goes out as its last
 *    handshake frame, and the connection is sealed in the same turn:
 *    what this end sends under the connector's key, what it receives
 *    under the connectee's. Nothing the daemon sends after the proof
 *    is read in the clear.
 *
 * Every failure closes the socket and throws the `HandshakeError`;
 * nothing is sent after it, since a failed handshake is a close and
 * nothing else in either direction. The router starts only once this
 * resolves, so no auth frame ever reaches it.
 */
export async function authorize(connection: Connection, authenticator: Authenticator): Promise<Peer> {
  try {
    // Step 1.
    const presented = await authenticator.present().catch((cause: unknown) => {
      throw new HandshakeError("present", cause);
    });
    let pair: CryptoKeyPair;
    let ours: Uint8Array;
    try {
      pair = (await crypto.subtle.generateKey(CURVE, false, ["deriveBits"])) as CryptoKeyPair;
      ours = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
    } catch (cause) {
      throw new HandshakeError("exchange", cause);
    }
    connection.send(encodeClientFrame({ kind: "auth", payload: offer(ours, presented.key, presented.token) }));
    // Step 2.
    const bytes = await connection.frames.next();
    if (bytes === undefined) {
      throw new HandshakeError("ended");
    }
    let answer: Answer;
    try {
      const frame = decodeServerFrame(bytes);
      if (frame.kind !== "auth") {
        throw new HandshakeError("not_auth");
      }
      answer = readAnswer(frame.payload);
    } catch (cause) {
      throw cause instanceof HandshakeError ? cause : new HandshakeError("malformed", cause);
    }
    // Step 3.
    const judged = await authenticator.verify(answer.token).catch((cause: unknown) => {
      throw new HandshakeError("refused", cause);
    });
    // Step 4.
    const named = await thumbprint(answer.key);
    if (named.length !== judged.thumbprint.length || !named.every((byte, at) => byte === judged.thumbprint[at])) {
      throw new HandshakeError("key");
    }
    // Step 5.
    const said = transcript(ours, answer.exchange, presented.token, answer.token);
    if (!(await proven(said, "connectee", answer.key, answer.signature))) {
      throw new HandshakeError("signature");
    }
    // Step 6.
    let shared: Uint8Array;
    try {
      if (answer.exchange[0] !== 4) {
        throw new HandshakeError("exchange");
      }
      const theirs = await crypto.subtle.importKey("raw", owned(answer.exchange), CURVE, false, []);
      shared = new Uint8Array(await crypto.subtle.deriveBits({ name: "ECDH", public: theirs }, pair.privateKey, 256));
    } catch (cause) {
      throw cause instanceof HandshakeError ? cause : new HandshakeError("exchange", cause);
    }
    const { connector, connectee } = await seals(said, shared);
    const signature = await presented.sign(signed(said, "connector")).catch((cause: unknown) => {
      throw new HandshakeError("present", cause);
    });
    // Step 7: the proof and the sealing, with nothing awaited between.
    connection.send(encodeClientFrame({ kind: "auth", payload: proof(signature) }));
    connection.seal(connector, connectee);
    return judged.peer;
  } catch (cause) {
    connection.close();
    throw cause instanceof HandshakeError ? cause : new HandshakeError("send", cause);
  }
}

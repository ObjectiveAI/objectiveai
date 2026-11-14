// The step in front: the handshake, as the connector.

import type { Connection } from "../connection.js";
import { KEY_LEN, offer, proof, readAnswer } from "../frame/auth.js";
import type { Answer } from "../frame/auth.js";
import { encodeClientFrame } from "../frame/client.js";
import { decodeServerFrame } from "../frame/server.js";
import { owned } from "../seal.js";
import type { Authenticator, Peer } from "./authenticator.js";
import { HandshakeError } from "./handshake_error.js";
import { proven, seals, signed, transcript } from "./transcript.js";

/**
 * The handshake from the end that dialled:
 *
 * 1. This end's token and signer, from `authenticator`, and a
 *    one-time X25519 key pair made for this connection. The pair's
 *    public half and the token go out as the first frame.
 * 2. The daemon's frame must be an auth frame, well formed: its
 *    one-time key, its signature, its token.
 * 3. That token is verified by `authenticator`, which answers the
 *    key it names. Its issuer need not be this end's.
 * 4. The daemon's signature is checked against that key, over the
 *    transcript: the daemon holds the key its token names, or the
 *    handshake ends.
 * 5. The shared secret is agreed, and the two session keys drawn. A
 *    secret that is all zero ends the handshake.
 * 6. This end's signature over the transcript goes out as its last
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
    // A runtime whose Web Crypto has no X25519 can make no handshake.
    let pair: CryptoKeyPair;
    let ours: Uint8Array;
    try {
      pair = (await crypto.subtle.generateKey({ name: "X25519" }, false, ["deriveBits"])) as CryptoKeyPair;
      ours = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
    } catch (cause) {
      throw new HandshakeError("exchange", cause);
    }
    connection.send(encodeClientFrame({ kind: "auth", payload: offer(ours, presented.token) }));
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
    const said = transcript(ours, answer.key, presented.token, answer.token);
    if (!(await proven(said, "connectee", judged.key, answer.signature))) {
      throw new HandshakeError("signature");
    }
    // Step 5.
    let shared: Uint8Array;
    try {
      const theirs = await crypto.subtle.importKey("raw", owned(answer.key), { name: "X25519" }, false, []);
      shared = new Uint8Array(await crypto.subtle.deriveBits({ name: "X25519", public: theirs }, pair.privateKey, 8 * KEY_LEN));
    } catch (cause) {
      throw new HandshakeError("exchange", cause);
    }
    if (shared.every((byte) => byte === 0)) {
      throw new HandshakeError("exchange");
    }
    const { connector, connectee } = await seals(said, shared);
    const signature = await presented.sign(signed(said, "connector")).catch((cause: unknown) => {
      throw new HandshakeError("present", cause);
    });
    // Step 6: the proof and the sealing, with nothing awaited between.
    connection.send(encodeClientFrame({ kind: "auth", payload: proof(signature) }));
    connection.seal(connector, connectee);
    return judged.peer;
  } catch (cause) {
    connection.close();
    throw cause instanceof HandshakeError ? cause : new HandshakeError("send", cause);
  }
}

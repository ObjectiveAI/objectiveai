// What an end supplies to the handshake.

/** The other end, verified: its token's `iss` and `sub`. */
export interface Peer {
  /** The token's `iss`. */
  readonly issuer: string;
  /** The token's `sub`. */
  readonly subject: string;
}

/**
 * What this end presents: its token, and what signs as the key the
 * token names. The two go together: a token presented with a signer
 * of another key proves nothing, and the other end closes.
 */
export interface Presented {
  /** The token: an ID token whose `cnf` claim names the key `sign` signs with. */
  readonly token: string;
  /** The Ed25519 signature of `message`, sixty-four bytes, by that key's private half. The handshake asks for one signature, and never for the key. */
  sign(message: Uint8Array): Promise<Uint8Array>;
}

/** The other end, judged: the identity its token establishes, and the key the token names, which the handshake then holds the other end to. */
export interface Judged {
  /** The identity. */
  readonly peer: Peer;
  /** The Ed25519 public key the token names in its `cnf` claim: thirty-two bytes. */
  readonly key: Uint8Array;
}

/**
 * This end's side of the handshake: its token and its key to
 * present, and its judgment of the other end's token. A rejection is
 * a reason for the caller's log; it never reaches the wire, where a
 * failed handshake is a close and nothing else.
 *
 * `verify` judges the token: its issuer, its signature by that
 * issuer's keys, its expiry, whom it names — and reads the key it
 * names. A token that names no key is not accepted. The handshake,
 * not this, then checks that the other end holds the key; a `verify`
 * that answers a key it did not read from a token it verified gives
 * that check nothing to stand on.
 */
export interface Authenticator {
  /** This end's token, good for presenting now, and what signs as the key it names: the identity it connects as. */
  present(): Promise<Presented>;
  /** The other end's `token`, judged: the identity it establishes, and the key it names. */
  verify(token: string): Promise<Judged>;
}

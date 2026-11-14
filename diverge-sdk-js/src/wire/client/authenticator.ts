// What an end supplies to the handshake.

/** The other end, verified: its token's `iss` and `sub`. */
export interface Peer {
  /** The token's `iss`. */
  readonly issuer: string;
  /** The token's `sub`. */
  readonly subject: string;
}

/**
 * What this end presents: its token, the public key the token is
 * bound to, and what signs with that key's private half. The three go
 * together: a token presented with another key, or a key with a
 * signer of another, proves nothing, and the other end closes.
 */
export interface Presented {
  /** The token: an access token bound to `key` as RFC 9449 binds one, its `cnf.jkt` the key's thumbprint. */
  readonly token: string;
  /** The public key the token is bound to: a P-256 point in the uncompressed form, sixty-five bytes. */
  readonly key: Uint8Array;
  /** The ECDSA signature of `message` over P-256 with SHA-256, the 32 bytes of `r` then the 32 bytes of `s`, by that key's private half. The handshake asks for one signature, and never for the key. */
  sign(message: Uint8Array): Promise<Uint8Array>;
}

/** The other end, judged: the identity its token establishes, and the thumbprint of the key the token is bound to, which the handshake then holds the other end's key to. */
export interface Judged {
  /** The identity. */
  readonly peer: Peer;
  /** The thumbprint the token names in its `cnf.jkt`: thirty-two bytes. */
  readonly thumbprint: Uint8Array;
}

/**
 * This end's side of the handshake: its token and its key to
 * present, and its judgment of the other end's token. A rejection is
 * a reason for the caller's log; it never reaches the wire, where a
 * failed handshake is a close and nothing else.
 *
 * `verify` judges the token: its issuer, its signature by that
 * issuer's keys, its expiry, whom it names — and reads the thumbprint
 * it is bound to. A token bound to no key is not accepted. The
 * handshake, not this, then checks that the key the other end sent
 * is the one named and that the other end holds it; a `verify` that
 * answers a thumbprint it did not read from a token it verified gives
 * that check nothing to stand on.
 */
export interface Authenticator {
  /** This end's token, good for presenting now, the key it is bound to, and what signs with that key: the identity it connects as. */
  present(): Promise<Presented>;
  /** The other end's `token`, judged: the identity it establishes, and the thumbprint it names. */
  verify(token: string): Promise<Judged>;
}

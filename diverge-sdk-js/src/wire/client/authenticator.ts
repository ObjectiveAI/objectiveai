// What an end supplies to the handshake.

/** The other end, verified: its token's `iss` and `sub`. */
export interface Peer {
  /** The token's `iss`. */
  readonly issuer: string;
  /** The token's `sub`. */
  readonly subject: string;
}

/**
 * This end's side of the handshake: its token to present, and its
 * judgment of the other end's. A rejection is a reason for the
 * caller's log; it never reaches the wire, where a failed handshake
 * is a close and nothing else.
 */
export interface Authenticator {
  /** This end's token from `issuer`, good for presenting now. */
  present(issuer: string): Promise<string>;
  /** The other end's `token`, judged: the identity it establishes. */
  verify(token: string): Promise<Peer>;
}

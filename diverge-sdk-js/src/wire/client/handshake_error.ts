// Why a handshake did not complete.

/**
 * Which step: the daemon gone before its handshake frame; its frame
 * not an auth frame; its frame unreadable; this end with no token to
 * present; the daemon's token not accepted; the daemon's signature
 * not its token's key's over this handshake; a key exchange that
 * yielded no shared secret; or a frame that could not be sent.
 */
export type HandshakeErrorKind = "ended" | "not_auth" | "malformed" | "present" | "refused" | "signature" | "exchange" | "send";

/**
 * A handshake that did not complete, and the socket closed with it.
 * None of it reached the daemon, which sees only the close.
 */
export class HandshakeError extends Error {
  /** Which step. */
  readonly kind: HandshakeErrorKind;

  constructor(kind: HandshakeErrorKind, cause?: unknown) {
    super(`the handshake did not complete: ${kind.replace("_", " ")}`, cause === undefined ? undefined : { cause });
    this.name = "HandshakeError";
    this.kind = kind;
  }
}

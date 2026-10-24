// Why a handshake did not complete.

/**
 * Which step: the daemon gone before its token; its first frame not
 * an auth frame; its token unreadable; this end with no token to
 * present; the daemon's token not accepted; an answer from another
 * issuer than the one presented; or a token that could not be sent.
 */
export type HandshakeErrorKind = "ended" | "not_auth" | "malformed" | "present" | "refused" | "issuer" | "send";

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

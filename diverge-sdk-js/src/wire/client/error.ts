// What a send can fail with.

/** Why a frame could not be sent. */
export type SendErrorKind = "connection" | "scope";

/**
 * The write failed — the socket is closed or refused the message,
 * and a socket that has errored is done, so this is a reason to stop
 * rather than retry — or the scope a channel was asked on is not
 * open: a scope ends when its finish arrives, and that can happen
 * between reading the last response and asking for a channel, so it
 * is an answer rather than a bug.
 */
export class SendError extends Error {
  /** Which. */
  readonly kind: SendErrorKind;

  constructor(kind: SendErrorKind, cause?: unknown) {
    super(kind === "connection" ? "the frame could not be written" : "the scope is not open", cause === undefined ? undefined : { cause });
    this.name = "SendError";
    this.kind = kind;
  }
}

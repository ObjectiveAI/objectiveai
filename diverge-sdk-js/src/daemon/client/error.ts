// Why an exchange did not get its answer.

/** Why a scope could not be opened. */
export type OpenErrorKind = "send" | "request";

/**
 * Before a frame is read, only two things can go wrong: the request
 * would not encode, or the frame would not send.
 */
export class OpenError extends Error {
  /** Which. */
  readonly kind: OpenErrorKind;

  constructor(kind: OpenErrorKind, cause: unknown) {
    super(kind === "send" ? "the request could not be sent" : "the request could not be encoded", { cause });
    this.name = "OpenError";
    this.kind = kind;
  }
}

/** Why a stream ended with no finish, or a frame could not be read. */
export type StreamErrorKind = "closed" | "frame" | "misrouted" | "response";

/**
 * The connection ended before the finish; a frame whose envelope
 * could not be read; a frame of a type no scope answer carries,
 * routed here by mistake; or a response whose payload is not what
 * the endpoint reads.
 */
export class StreamError extends Error {
  /** Which. */
  readonly kind: StreamErrorKind;

  constructor(kind: StreamErrorKind, cause?: unknown) {
    super(`the stream failed: ${kind}`, cause === undefined ? undefined : { cause });
    this.name = "StreamError";
    this.kind = kind;
  }
}

/** Why a one-answer exchange got no answer. */
export type OneShotErrorKind = "send" | "request" | "closed" | "frame" | "unanswered" | "misrouted" | "response";

/**
 * The open failing, as `OpenError` says; the connection ending
 * first; a frame that could not be read; the daemon finishing the
 * scope without answering — a request it could not read; a frame
 * routed here by mistake; or an answer that is not what the
 * endpoint reads.
 */
export class OneShotError extends Error {
  /** Which. */
  readonly kind: OneShotErrorKind;

  constructor(kind: OneShotErrorKind, cause?: unknown) {
    super(`the exchange got no answer: ${kind}`, cause === undefined ? undefined : { cause });
    this.name = "OneShotError";
    this.kind = kind;
  }
}

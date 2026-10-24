// Why a channel did not get its answer.

/** Why a channel could not be opened. */
export type OpenErrorKind = "request" | "send";

/** The request would not encode, or the frame would not send. */
export class OpenError extends Error {
  /** Which. */
  readonly kind: OpenErrorKind;

  constructor(kind: OpenErrorKind, cause: unknown) {
    super(kind === "send" ? "the channel request could not be sent" : "the channel request could not be encoded", { cause });
    this.name = "OpenError";
    this.kind = kind;
  }
}

/** Why a unary channel got no answer, or a refusal. */
export type UnaryErrorKind = "request" | "send" | "closed" | "frame" | "unanswered" | "misrouted" | "response" | "refused";

/**
 * A one-answer channel that did not get its answer: the open
 * failing; the connection ending first; a frame that could not be
 * read; the channel finishing without answering; a frame routed
 * here by mistake; an answer that is not what the exchange reads;
 * or the server's refusal — the answer that says no, carried as
 * `refusal`, which is an answer and not a failure of the machinery.
 */
export class UnaryError<R> extends Error {
  /** Which. */
  readonly kind: UnaryErrorKind;
  /** The server's refusal, on `refused`. */
  readonly refusal: R | undefined;

  constructor(kind: UnaryErrorKind, cause?: unknown, refusal?: R) {
    super(`the channel got no answer: ${kind}`, cause === undefined ? undefined : { cause });
    this.name = "UnaryError";
    this.kind = kind;
    this.refusal = refusal;
  }
}

/** Why a channel stream ended with no finish. */
export type ChannelStreamErrorKind = "closed" | "frame" | "misrouted" | "response" | "refused";

/**
 * A channel stream that ended other than by its finish: the
 * connection ending; a frame that could not be read; a frame routed
 * here by mistake; an item that is not what the exchange reads; or
 * the server's refusal, carried as `refusal`, the last thing on the
 * channel. Every one is terminal.
 */
export class ChannelStreamError<R> extends Error {
  /** Which. */
  readonly kind: ChannelStreamErrorKind;
  /** The server's refusal, on `refused`. */
  readonly refusal: R | undefined;

  constructor(kind: ChannelStreamErrorKind, cause?: unknown, refusal?: R) {
    super(`the channel stream failed: ${kind}`, cause === undefined ? undefined : { cause });
    this.name = "ChannelStreamError";
    this.kind = kind;
    this.refusal = refusal;
  }
}

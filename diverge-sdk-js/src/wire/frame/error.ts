// What can be wrong with a frame's envelope.

/**
 * Why a frame could not be read: the two envelope failures, and an
 * auth frame whose token is empty or not UTF-8.
 */
export type FrameErrorKind = "truncated" | "unknown_type" | "auth";

/**
 * A frame shorter than its header, one whose type this side does not
 * define — which includes a type the OTHER side is allowed to send
 * and this one is not — or an auth frame with no token in it.
 * Nothing else is an envelope error: the payload is the endpoint's
 * business.
 */
export class FrameError extends Error {
  /** Which of the three. */
  readonly kind: FrameErrorKind;
  /** The type byte, for `unknown_type`. */
  readonly type: number | undefined;

  constructor(kind: FrameErrorKind, type?: number, cause?: unknown) {
    super(
      kind === "truncated"
        ? "the frame is shorter than its header"
        : kind === "unknown_type"
          ? `the frame type ${type} is not one this side reads`
          : "the auth frame's token is empty or not UTF-8",
      cause === undefined ? undefined : { cause },
    );
    this.name = "FrameError";
    this.kind = kind;
    this.type = type;
  }
}

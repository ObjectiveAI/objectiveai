// What can be wrong with a frame's envelope.

/** Why a frame could not be read: the two envelope failures. */
export type FrameErrorKind = "truncated" | "unknown_type";

/**
 * A frame shorter than its header, or one whose type this side does
 * not define — which includes a type the OTHER side is allowed to
 * send and this one is not. Nothing else is an envelope error: the
 * payload is the endpoint's business.
 */
export class FrameError extends Error {
  /** Which of the two. */
  readonly kind: FrameErrorKind;
  /** The type byte, for `unknown_type`. */
  readonly type: number | undefined;

  constructor(kind: FrameErrorKind, type?: number) {
    super(kind === "truncated" ? "the frame is shorter than its header" : `the frame type ${type} is not one this side reads`);
    this.name = "FrameError";
    this.kind = kind;
    this.type = type;
  }
}

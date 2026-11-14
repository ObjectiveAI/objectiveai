// Why a FUSE frame could not be read or written.

/** Which rule a FUSE frame broke. */
export type FuseErrorKind = "empty" | "unknown_kind" | "truncated" | "utf8" | "length";

/**
 * A FUSE frame that is not one: no bytes at all; a leading byte that
 * names no variant; fewer bytes than the frame promises; a path, a
 * name or a message that is not UTF-8; or — writing one — a path or a
 * name longer than the 65535 bytes its two-byte length can state.
 */
export class FuseError extends Error {
  /** Which. */
  readonly kind: FuseErrorKind;

  constructor(kind: FuseErrorKind, detail?: number) {
    super(detail === undefined ? `the fuse frame is not one: ${kind}` : `the fuse frame is not one: ${kind} (${detail})`);
    this.name = "FuseError";
    this.kind = kind;
  }
}

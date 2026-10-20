// A `_meta` without the attestation expected.

import { KEY } from "../meta.js";

/** Why the attestation could not be read. */
export type AttestationErrorKind = "missing" | "invalid";

/**
 * The `_meta` has no attestation, or has one that is not what this
 * side expects there — a tool's where a caller's was asked for, or
 * a value of another shape.
 */
export class AttestationError extends Error {
  /** Which. */
  readonly kind: AttestationErrorKind;

  constructor(kind: AttestationErrorKind, cause?: unknown) {
    super(kind === "missing" ? `\`_meta\` has no \`${KEY}\`` : `\`_meta\`'s \`${KEY}\` is not what this side is attested`, cause === undefined ? undefined : { cause });
    this.name = "AttestationError";
    this.kind = kind;
  }
}

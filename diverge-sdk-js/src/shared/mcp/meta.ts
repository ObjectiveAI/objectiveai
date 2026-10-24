// The key under which an attestation rides a `_meta`.

/**
 * The one key this SDK claims in MCP's `_meta`: the attestation of
 * who is asking, on a request, or who answered, on a response. A
 * dotted name, as MCP reserves `_meta` keys to their owners.
 */
export const KEY = "diverge.network";

/** A `_meta` object: MCP's general field, any members. */
export type Meta = Record<string, unknown>;

/** Put an attestation on a `_meta`, replacing one there. */
export function attest(meta: Meta, attestation: unknown): void {
  meta[KEY] = attestation;
}

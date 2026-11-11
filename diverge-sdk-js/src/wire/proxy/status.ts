// How the link stands, as the app's Rust says it.

import { z } from "zod";

/**
 * How the app's link to the daemon stands: not yet dialled, being
 * dialled, up, or down with the reason. `failed` is the connection
 * having ended or the dial having failed; every send is refused
 * `connection` until `reset`. The Rust SDK's
 * `diverge_sdk::wire::proxy::WireStatus`, as it serializes.
 */
export const WireStatus = z.discriminatedUnion("state", [
  /** Nothing has used the link since it was made or reset. */
  z.object({ state: z.literal("idle") }),
  /** A use is dialling now. */
  z.object({ state: z.literal("dialling") }),
  /** The connection is up. */
  z.object({ state: z.literal("connected") }),
  /** The last dial failed, or the connection ended. */
  z.object({ state: z.literal("failed"), message: z.string() }),
]);
export type WireStatus = z.infer<typeof WireStatus>;

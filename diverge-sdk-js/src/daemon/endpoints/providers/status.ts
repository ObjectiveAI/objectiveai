// Where the daemon stands with a provider.

import { z } from "zod";
import { Frame as Info } from "../../../provider/endpoints/info/server/response/frame.js";

/**
 * The daemon's standing with one provider, as a list or a get
 * reports it: no connection; a connection the daemon uses, with what
 * the provider answered the daemon's `info` ask, cached for the
 * connection's life; or a connection the daemon holds open and never
 * uses, the provider having answered a version that is not the
 * daemon's own. An object tagged by `type`.
 */
export const Status = z.discriminatedUnion("type", [
  /** The daemon holds no connection to the provider. */
  z.object({ type: z.literal("disconnected") }),
  /** The daemon holds a connection it uses, and this is what the provider answered its `info` ask. */
  z.object({ type: z.literal("connected"), info: Info }),
  /** The daemon holds a connection it never uses: the provider's version is not the daemon's. */
  z.object({ type: z.literal("different_version"), info: Info }),
]);
export type Status = z.infer<typeof Status>;

/** Whether the daemon uses the provider now: `connected`, and nothing else. */
export function connected(status: Status): boolean {
  return status.type === "connected";
}

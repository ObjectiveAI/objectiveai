// The tool's run, while its container is up.

import { z } from "zod";
import { Provider } from "../../../../../agent_log/provider.js";

/**
 * The tool's container, while it runs: where, and under what id. A tool
 * has at most one run — a transient tool's for its agent's run, a
 * service's for as long as the daemon keeps it up — and none while its
 * container is down: a service held between runs has no run until it
 * runs again.
 */
export const Run = z.object({
  /** The provider the container runs on. See `Provider`. */
  provider: Provider,
  /** The container's id, as the provider's run answered it. */
  id: z.string(),
});
export type Run = z.infer<typeof Run>;

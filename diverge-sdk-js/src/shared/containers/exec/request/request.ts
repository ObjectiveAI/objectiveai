// One command to run in a container.

import { z } from "zod";

/**
 * Run one command in the container: an argv run as it is, no shell
 * between — the first item a path, or a name the container's `PATH`
 * finds — its stdin the null device. A caller that wants a shell
 * names it, `["sh", "-c", "…"]`.
 */
export const Request = z.object({
  /** The command and its arguments. At least one item. */
  argv: z.array(z.string()).min(1),
  /** Where it runs, as path components from the container's root; absent, the image's `WORKDIR`. */
  cwd: z.array(z.string()).optional(),
  /** Variables set for it, over the container's own environment; absent, none. */
  env: z.record(z.string(), z.string()).optional(),
});
export type Request = z.infer<typeof Request>;

// Transient, or a service.

import { z } from "zod";

/**
 * Transient or a service: the `mode` a dependency template is tagged
 * with, as a value of its own to narrow a list by. Snake case on the
 * wire, as the tag is: `"transient"`, `"service"`.
 */
export const Mode = z.enum(["transient", "service"]);
export type Mode = z.infer<typeof Mode>;

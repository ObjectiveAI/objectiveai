// How a tool made from a template is kept running.

import { z } from "zod";

/**
 * Transient or a service: the `mode` a tool template says beside
 * its `type`. A transient tool runs while it is used; a service is
 * kept while anything holds it. Snake case on the wire:
 * `"transient"`, `"service"`; absent is transient.
 */
export const Mode = z.enum(["transient", "service"]);
export type Mode = z.infer<typeof Mode>;

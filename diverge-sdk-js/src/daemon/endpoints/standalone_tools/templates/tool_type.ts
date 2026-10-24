// What tells a tool template from an agent template.

import { z } from "zod";
import { Mode } from "./mode.js";

/**
 * `"type":"tool"`, and the `mode` beside it: what tells a tool
 * template from an agent template, and how every tool made from it
 * is kept. Flat among the template's members; a template that says
 * no `mode` is transient.
 */
export const ToolType = z.object({
  type: z.literal("tool"),
  /** How a tool made from the template is kept running. Absent is transient. */
  mode: Mode.default("transient"),
});
export type ToolType = z.infer<typeof ToolType>;

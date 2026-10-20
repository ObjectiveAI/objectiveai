// A tool template.

import { z } from "zod";
import { template } from "../../../template/template.js";
import { ToolType } from "./tool_type.js";

/** A tool template: the daemon's one `template`, typed `ToolType`. */
export const Template = template(ToolType);
export type Template = z.infer<typeof Template>;

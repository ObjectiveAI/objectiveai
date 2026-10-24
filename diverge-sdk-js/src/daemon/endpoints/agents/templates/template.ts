// An agent template.

import { z } from "zod";
import { template } from "../../../template/template.js";
import { AgentType } from "./agent_type.js";

/** An agent template: the daemon's one `template`, typed `AgentType`. */
export const Template = template(AgentType);
export type Template = z.infer<typeof Template>;

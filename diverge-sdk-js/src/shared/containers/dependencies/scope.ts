// How far a service is shared.

import { z } from "zod";

/**
 * What a service tool is one of: one per parent agent, one per
 * parent agent template, or one for every agent that declares it,
 * whatever template they are made from. The scope names the service
 * — the same template declared under the same scope key is the same
 * service, joined and not made again — and names its database scope
 * with it. Snake case on the wire: `"instance"`, `"template"`,
 * `"global"`.
 */
export const Scope = z.enum(["instance", "template", "global"]);
export type Scope = z.infer<typeof Scope>;

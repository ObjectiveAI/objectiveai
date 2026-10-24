// How a tool came to be, and what the daemon knows of it.

import { z } from "zod";
import { Provider } from "../../../../../../daemon/endpoints/agents/logs/server/response/provider.js";
import { Mode } from "../../../../../../daemon/endpoints/standalone_tools/templates/mode.js";
import * as reference from "../../../../../../daemon/reference/tool.js";

/**
 * The two ways a tool comes to be, and what the daemon knows of each: a
 * `create` made it from an image, or a `register` named a tool another
 * daemon holds. JSON-tagged by `kind`, `created` or `connected`.
 *
 * Either is a record, with the index its kind counts: the first made
 * the same way is `1`, each after is one more, and no number is given
 * twice, so the fixed part and the index name the tool once and for
 * all. A dependency tool — one running from a template an agent's
 * program declared — is no tool of this family: it is listed under its
 * template by the `dependency_tools` list, and nowhere here.
 */
export const Origin = z.discriminatedUnion("kind", [
  /** Made by a create, from a template: which, its number among the tools ever made from it, how it is kept running, the provider it runs on once it has, and its container's id while it runs. */
  z.object({
    kind: z.literal("created"),
    /** The template it was made from, by id. */
    template: z.string(),
    /** Its number among all created tools of the caller's ever made from that template, deleted ones included; never given twice. */
    index: z.number().int().nonnegative(),
    /** How it is kept running, as the template says. */
    mode: Mode,
    /** The provider its container runs on, once it has run; absent before. */
    provider: Provider.optional(),
    /** Its container's id on that provider, while it runs; absent otherwise. */
    id: z.string().optional(),
  }),
  /** Registered, from another daemon's tool: the remote daemon, the tool as that daemon names it, and its number among the tools ever registered. */
  z.object({
    kind: z.literal("connected"),
    /** The remote daemon the tool is on, by its name in the configuration, as the register named it. */
    daemon: z.string(),
    /** The tool, as that daemon names it. */
    tool: reference.Tool,
    /** Its number among all connected tools of the caller's ever registered, deleted ones included; never given twice. */
    index: z.number().int().nonnegative(),
  }),
]);
export type Origin = z.infer<typeof Origin>;

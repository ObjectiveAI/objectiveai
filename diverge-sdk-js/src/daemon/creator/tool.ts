// A tool as a creator.

import { z } from "zod";
import { Owner } from "../key/owner.js";

/** A tool the client made from a template. */
export const Record = z.object({
  /** The template the tool was made from, by id. */
  template: z.string(),
  /** The tool's number among all tools of the client's ever made from that template. */
  index: z.number().int().nonnegative(),
  /** The tool's name, as its create gave it, if it gave one. Absent when it had none. */
  name: z.string().optional(),
});
export type Record = z.infer<typeof Record>;

/** A dependency tool, running from a template an agent's program declared. */
export const Dependency = z.object({
  /** The dependency template it runs from, by id. */
  template: z.string(),
  /** Whose it is: see `key.Owner`. */
  owner: Owner,
});
export type Dependency = z.infer<typeof Dependency>;

/**
 * A tool of the client's that made something: one the client made
 * from a template, or a dependency tool. A connected tool makes
 * nothing. Externally tagged on the wire: `{"record":{…}}` or
 * `{"dependency":{…}}`, beside the creator's own `type`.
 */
export const Tool = z.union([z.strictObject({ record: Record }), z.strictObject({ dependency: Dependency })]);
export type Tool = z.infer<typeof Tool>;

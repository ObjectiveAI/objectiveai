// One operation of a patch.

import { z } from "zod";

/**
 * One operation of RFC 6902, on one place in the document, tagged
 * by `op`. `path` and `from` are JSON Pointers, RFC 6901: the empty
 * string is the whole document, `/a/b` the member `b` of `a`,
 * `/list/0` the first element, `/list/-` the place after its last,
 * which only an `add` may name. `add` puts the value at the path;
 * `remove` takes away what is there; `replace` puts the value where
 * one already is; `move` and `copy` take from `from`; `test` fails
 * the patch unless what is at the path equals the value.
 */
export const Operation = z.discriminatedUnion("op", [
  z.object({ op: z.literal("add"), path: z.string(), value: z.json() }),
  z.object({ op: z.literal("remove"), path: z.string() }),
  z.object({ op: z.literal("replace"), path: z.string(), value: z.json() }),
  z.object({ op: z.literal("move"), from: z.string(), path: z.string() }),
  z.object({ op: z.literal("copy"), from: z.string(), path: z.string() }),
  z.object({ op: z.literal("test"), path: z.string(), value: z.json() }),
]);
export type Operation = z.infer<typeof Operation>;

/** A patch: operations, applied in order, whole or not at all. */
export const Patch = z.array(Operation);
export type Patch = z.infer<typeof Patch>;

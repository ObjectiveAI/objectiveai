// One entry of a filetree.

import { z } from "zod";
import { Reader, Writer } from "../../../wire/postcard.js";

/**
 * One node of the tree, as JSON writes it: tagged by `type`, snake
 * case. A file has its size; a directory its children, recursively,
 * and `changes`, whether changes beneath it stream — `false` marks a
 * subtree the source could not watch, which a consumer treats as
 * possibly stale; a symlink its target as components relative to the
 * filetree root, the same frame of reference as a frame's `path`,
 * and the link is never followed. Times are unsigned Unix seconds,
 * `null` where the filesystem did not say.
 */
export type Node =
  | { type: "file"; name: string; size: number | null; created_at: number | null; modified_at: number | null }
  | { type: "directory"; name: string; created_at: number | null; modified_at: number | null; changes: boolean; children: Node[] }
  | { type: "symlink"; name: string; path: string[]; created_at: number | null; modified_at: number | null };

const time = z.number().int().nonnegative().nullable();

export const Node: z.ZodType<Node> = z.lazy(() =>
  z.discriminatedUnion("type", [
    z.object({ type: z.literal("file"), name: z.string(), size: time, created_at: time, modified_at: time }),
    z.object({
      type: z.literal("directory"),
      name: z.string(),
      created_at: time,
      modified_at: time,
      changes: z.boolean(),
      children: z.array(Node),
    }),
    z.object({ type: z.literal("symlink"), name: z.string(), path: z.array(z.string()), created_at: time, modified_at: time }),
  ]),
);

// On the wire a node is postcard, which knows a variant by its INDEX:
// file 0, directory 1, symlink 2, members in declaration order.
// Reordering these, or inserting one among them, silently changes
// what existing bytes mean; new variants go on the end.
const FILE = 0;
const DIRECTORY = 1;
const SYMLINK = 2;

/** Write one node, postcard. */
export function writeNode(writer: Writer, node: Node): void {
  const time = (w: Writer, t: number) => w.varint(t);
  switch (node.type) {
    case "file":
      writer.varint(FILE).string(node.name).option(node.size, time).option(node.created_at, time).option(node.modified_at, time);
      return;
    case "directory":
      writer
        .varint(DIRECTORY)
        .string(node.name)
        .option(node.created_at, time)
        .option(node.modified_at, time)
        .bool(node.changes)
        .seq(node.children, writeNode);
      return;
    case "symlink":
      writer
        .varint(SYMLINK)
        .string(node.name)
        .seq(node.path, (w, p) => w.string(p))
        .option(node.created_at, time)
        .option(node.modified_at, time);
      return;
  }
}

/** Read one node, postcard. An index past two is a `PostcardError`. */
export function readNode(reader: Reader): Node {
  const time = (r: Reader) => r.varint();
  switch (reader.varint()) {
    case FILE:
      return { type: "file", name: reader.string(), size: reader.option(time), created_at: reader.option(time), modified_at: reader.option(time) };
    case DIRECTORY:
      return {
        type: "directory",
        name: reader.string(),
        created_at: reader.option(time),
        modified_at: reader.option(time),
        changes: reader.bool(),
        children: reader.seq(readNode),
      };
    case SYMLINK:
      return {
        type: "symlink",
        name: reader.string(),
        path: reader.seq((r) => r.string()),
        created_at: reader.option(time),
        modified_at: reader.option(time),
      };
    default:
      throw new RangeError("a node variant past the three");
  }
}

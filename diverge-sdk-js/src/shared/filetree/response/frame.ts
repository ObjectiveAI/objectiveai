// One change on a filetree stream.

import { z } from "zod";
import { Reader, Writer } from "../../../wire/postcard.js";
import { Node, readNode, writeNode } from "./node.js";

/**
 * One change on a filetree stream, as JSON writes it: tagged by
 * `type`. `snapshot` establishes the tree. Every other variant names
 * exactly one node by its path — components relative to the root —
 * and says what became of it: it appeared, changed in place, or
 * ceased to exist. A delta that carries a node carries its COMPLETE
 * value, never a patch; a directory inserted comes with its whole
 * subtree. There is no move: a rename is `removed` at the path it
 * left and `inserted` at the path it arrived at.
 */
export const Frame = z.discriminatedUnion("type", [
  z.object({ type: z.literal("snapshot"), children: z.array(Node) }),
  z.object({ type: z.literal("inserted"), path: z.array(z.string()), node: Node }),
  z.object({ type: z.literal("modified"), path: z.array(z.string()), node: Node }),
  z.object({ type: z.literal("removed"), path: z.array(z.string()) }),
]);
export type Frame = z.infer<typeof Frame>;

// Variant ORDER is part of the wire format: postcard writes the
// index. snapshot 0, inserted 1, modified 2, removed 3; new variants
// go on the end.
const SNAPSHOT = 0;
const INSERTED = 1;
const MODIFIED = 2;
const REMOVED = 3;

const path = (w: Writer, p: readonly string[]) => w.seq(p, (ww, c) => ww.string(c));

/** The frame's postcard bytes. */
export function encode(frame: Frame): Uint8Array {
  const writer = new Writer();
  switch (frame.type) {
    case "snapshot":
      writer.varint(SNAPSHOT).seq(frame.children, writeNode);
      break;
    case "inserted":
      writer.varint(INSERTED);
      path(writer, frame.path);
      writeNode(writer, frame.node);
      break;
    case "modified":
      writer.varint(MODIFIED);
      path(writer, frame.path);
      writeNode(writer, frame.node);
      break;
    case "removed":
      writer.varint(REMOVED);
      path(writer, frame.path);
      break;
  }
  return writer.finish();
}

/** A frame out of its postcard bytes, the whole payload consumed. */
export function decode(bytes: Uint8Array): Frame {
  const reader = new Reader(bytes);
  const components = (r: Reader) => r.seq((rr) => rr.string());
  let frame: Frame;
  switch (reader.varint()) {
    case SNAPSHOT:
      frame = { type: "snapshot", children: reader.seq(readNode) };
      break;
    case INSERTED:
      frame = { type: "inserted", path: components(reader), node: readNode(reader) };
      break;
    case MODIFIED:
      frame = { type: "modified", path: components(reader), node: readNode(reader) };
      break;
    case REMOVED:
      frame = { type: "removed", path: components(reader) };
      break;
    default:
      throw new RangeError("a frame variant past the four");
  }
  reader.end();
  return frame;
}

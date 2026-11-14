// The nine asks a FUSE mount makes.

import { z } from "zod";
import { concat } from "../../../../../../../shared/containers/fuse/bytes.js";
import { FuseError } from "../../../../../../../shared/containers/fuse/error.js";
import * as path from "./path.js";
import * as read from "./read.js";
import * as rename from "./rename.js";
import * as setattr from "./setattr.js";
import * as truncate from "./truncate.js";
import * as write from "./write.js";

/**
 * One ask of whoever serves a mount, each a channel of its own with
 * one answer. A payload leads with one byte saying which, and the
 * rest is that ask's own bytes.
 *
 * | tag | ask | answered with |
 * |-----|-----|---------------|
 * | `0` | `read`: a piece of a file | a `read` answer |
 * | `1` | `write`: a piece of a file, written | an `ack` |
 * | `2` | `list`: a directory's entries | a `list` answer |
 * | `3` | `remove`: a file, or an empty directory, gone | an `ack` |
 * | `4` | `rename`: one path moved to another | an `ack` |
 * | `5` | `mkdir`: a directory made | an `ack` |
 * | `6` | `stat`: what is at a path | a `stat` answer |
 * | `7` | `truncate`: a file's size set | an `ack` |
 * | `8` | `setattr`: a path's attributes changed | an `ack` |
 */
export const Frame = z.union([
  z.strictObject({ read: read.Read }),
  z.strictObject({ write: write.Write }),
  z.strictObject({ list: path.Path }),
  z.strictObject({ remove: path.Path }),
  z.strictObject({ rename: rename.Rename }),
  z.strictObject({ mkdir: path.Path }),
  z.strictObject({ stat: path.Path }),
  z.strictObject({ truncate: truncate.Truncate }),
  z.strictObject({ setattr: setattr.Setattr }),
]);
export type Frame = z.infer<typeof Frame>;

const READ = 0;
const WRITE = 1;
const LIST = 2;
const REMOVE = 3;
const RENAME = 4;
const MKDIR = 5;
const STAT = 6;
const TRUNCATE = 7;
const SETATTR = 8;

/** The ask as its bytes: the tag, then the ask's own. */
export function encode(frame: Frame): Uint8Array {
  const tagged = (tag: number, body: Uint8Array): Uint8Array => concat(Uint8Array.of(tag), body);
  if ("read" in frame) {
    return tagged(READ, read.encode(frame.read));
  }
  if ("write" in frame) {
    return tagged(WRITE, write.encode(frame.write));
  }
  if ("list" in frame) {
    return tagged(LIST, path.encode(frame.list));
  }
  if ("remove" in frame) {
    return tagged(REMOVE, path.encode(frame.remove));
  }
  if ("rename" in frame) {
    return tagged(RENAME, rename.encode(frame.rename));
  }
  if ("mkdir" in frame) {
    return tagged(MKDIR, path.encode(frame.mkdir));
  }
  if ("stat" in frame) {
    return tagged(STAT, path.encode(frame.stat));
  }
  if ("truncate" in frame) {
    return tagged(TRUNCATE, truncate.encode(frame.truncate));
  }
  return tagged(SETATTR, setattr.encode(frame.setattr));
}

/** An ask out of its bytes; a tag that names none is `unknown_kind`. */
export function decode(bytes: Uint8Array): Frame {
  if (bytes.length === 0) {
    throw new FuseError("empty");
  }
  const rest = bytes.subarray(1);
  switch (bytes[0]) {
    case READ:
      return { read: read.decode(rest) };
    case WRITE:
      return { write: write.decode(rest) };
    case LIST:
      return { list: path.decode(rest) };
    case REMOVE:
      return { remove: path.decode(rest) };
    case RENAME:
      return { rename: rename.decode(rest) };
    case MKDIR:
      return { mkdir: path.decode(rest) };
    case STAT:
      return { stat: path.decode(rest) };
    case TRUNCATE:
      return { truncate: truncate.decode(rest) };
    case SETATTR:
      return { setattr: setattr.decode(rest) };
    default:
      throw new FuseError("unknown_kind", bytes[0]);
  }
}

// The filetree's root, and the fold that keeps it current.

import type { Frame } from "./frame.js";
import type { Node } from "./node.js";

/**
 * The filetree's root — the whole tree, as the root's entries. The
 * root is deliberately NOT a node: it has no name, no metadata and no
 * path, and every path is expressed relative to it. It is written as
 * JSON as the bare list of its entries, and it is the shape a
 * consumer materializes: applying deltas means editing this list, so
 * what a consumer holds and what a snapshot delivers are the same.
 */
export class Root {
  /** The root's entries, recursively. Empty is an empty list. */
  children: Node[];

  constructor(children: Node[] = []) {
    this.children = children;
  }

  /**
   * Fold one frame into this tree.
   *
   * This is THE fold. Every holder of a materialized tree applies
   * frames the same way, so that two consumers fed the same stream
   * cannot disagree about what it meant.
   *
   * Lenient by design: a frame that cannot be applied — an empty
   * path, a parent that is missing or is not a directory — is
   * DROPPED, never guessed at. The fold does not fabricate the
   * directories a path implies: a tree missing a subtree is
   * recoverable from the next snapshot, whereas a tree containing
   * invented nodes is wrong in a way nothing detects.
   *
   * Replay-safe: `inserted` and `modified` both place a complete
   * node, so applying either twice is a no-op, and applying one
   * where the other was expected still lands the right value;
   * `removed` of a path already empty is nothing; a `snapshot`
   * replaces the tree whole whenever it comes.
   */
  update(frame: Frame): void {
    switch (frame.type) {
      case "snapshot":
        this.children = frame.children;
        return;
      case "inserted":
      case "modified":
        this.#place(frame.path, frame.node);
        return;
      case "removed":
        this.#detach(frame.path);
        return;
    }
  }

  /** The bare list, which is how a root is written. */
  toJSON(): Node[] {
    return this.children;
  }

  #place(path: readonly string[], node: Node): void {
    const leaf = path[path.length - 1];
    if (leaf === undefined) {
      return;
    }
    const siblings = descend(this.children, path.slice(0, -1));
    if (siblings === undefined) {
      return;
    }
    const at = siblings.findIndex((child) => child.name === leaf);
    if (at === -1) {
      siblings.push(node);
    } else {
      siblings[at] = node;
    }
  }

  #detach(path: readonly string[]): void {
    const leaf = path[path.length - 1];
    if (leaf === undefined) {
      return;
    }
    const siblings = descend(this.children, path.slice(0, -1));
    if (siblings === undefined) {
      return;
    }
    const at = siblings.findIndex((child) => child.name === leaf);
    if (at !== -1) {
      siblings.splice(at, 1);
    }
  }
}

/**
 * Walk `components` down from `children`, following each into its
 * directory's entries. `undefined` if a component is absent or names
 * something that is not a directory.
 */
function descend(children: Node[], components: readonly string[]): Node[] | undefined {
  let here = children;
  for (const component of components) {
    const next = here.find((child) => child.name === component);
    if (next === undefined || next.type !== "directory") {
      return undefined;
    }
    here = next.children;
  }
  return here;
}

// Apply the daemon's filetree frames to a tree. Every delta carries the
// complete node and names one place, so replaying one is harmless.

import type { FileNode } from "../bindings/FileNode";
import type { TreeEvent } from "../bindings/TreeEvent";

function put(nodes: FileNode[], path: string[], node: FileNode | null): FileNode[] {
  const [head, ...rest] = path;
  if (rest.length === 0) {
    const others = nodes.filter((n) => n.name !== head);
    return node ? [...others, node].sort((a, b) => a.name.localeCompare(b.name)) : others;
  }
  return nodes.map((n) => (n.name === head && n.kind === "directory" ? { ...n, children: put(n.children, rest, node) } : n));
}

export function applyTree(tree: FileNode[], event: TreeEvent): FileNode[] {
  switch (event.event) {
    case "snapshot":
      return event.children;
    case "inserted":
    case "modified":
      return put(tree, event.path, event.node);
    case "removed":
      return put(tree, event.path, null);
    case "error":
    case "end":
      return tree;
  }
}

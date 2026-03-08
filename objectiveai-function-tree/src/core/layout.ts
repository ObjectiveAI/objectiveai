import type { TreeNode, TreeData, FunctionTreeConfig } from "../types";
import { NODE_SIZES } from "../types";

// ---------------------------------------------------------------------------
// Reingold-Tilford-inspired tree layout
// ---------------------------------------------------------------------------

interface SubtreeInfo {
  /** Total width of this subtree (including gaps). */
  width: number;
  /** Total height of this subtree (including gaps). */
  height: number;
}

/**
 * Lay out the tree using a modified Reingold-Tilford algorithm.
 * - Root is placed at top center.
 * - Children are distributed horizontally, centered under their parent.
 * - Large fan-outs (> gridThreshold) use a grid layout instead of a row.
 *
 * Mutates node x/y positions in place.
 */
export function layoutTree(
  treeData: TreeData,
  config: FunctionTreeConfig
): void {
  const { nodes, rootId } = treeData;
  const root = nodes.get(rootId);
  if (!root) return;

  // Pass 1: Compute subtree sizes (bottom-up)
  const subtreeInfos = new Map<string, SubtreeInfo>();
  computeSubtreeSize(rootId, nodes, config, subtreeInfos);

  // Pass 2: Assign positions (top-down)
  const rootInfo = subtreeInfos.get(rootId);
  if (!rootInfo) return;

  // Center root at origin (0, 0)
  assignPositions(rootId, 0, 0, nodes, config, subtreeInfos);
}

function computeSubtreeSize(
  nodeId: string,
  nodes: Map<string, TreeNode>,
  config: FunctionTreeConfig,
  infos: Map<string, SubtreeInfo>
): SubtreeInfo {
  const node = nodes.get(nodeId);
  if (!node) return { width: 0, height: 0 };

  const children = node.children
    .map((cid) => nodes.get(cid))
    .filter((c): c is TreeNode => c !== undefined);

  // Leaf node
  if (children.length === 0) {
    const info: SubtreeInfo = { width: node.width, height: node.height };
    infos.set(nodeId, info);
    return info;
  }

  // Compute children subtree sizes first
  const childInfos: SubtreeInfo[] = [];
  for (const child of children) {
    childInfos.push(computeSubtreeSize(child.id, nodes, config, infos));
  }

  const useGrid =
    children.length > config.gridThreshold &&
    children.every((c) => c.children.length === 0);

  let childrenWidth: number;
  let childrenHeight: number;

  if (useGrid) {
    // Grid layout for large fan-outs of leaf nodes
    const cols = Math.ceil(Math.sqrt(children.length));
    const rows = Math.ceil(children.length / cols);
    const cellWidth = children[0].width;
    const cellHeight = children[0].height;
    childrenWidth = cols * cellWidth + (cols - 1) * config.nodeGapX;
    childrenHeight = rows * cellHeight + (rows - 1) * (config.nodeGapX * 0.5);
  } else {
    // Row layout: sum of children widths + gaps
    childrenWidth = childInfos.reduce((sum, ci) => sum + ci.width, 0) +
      (childInfos.length - 1) * config.nodeGapX;
    childrenHeight = Math.max(...childInfos.map((ci) => ci.height));
  }

  const totalWidth = Math.max(node.width, childrenWidth);
  const totalHeight = node.height + config.nodeGapY + childrenHeight;

  const info: SubtreeInfo = { width: totalWidth, height: totalHeight };
  infos.set(nodeId, info);
  return info;
}

function assignPositions(
  nodeId: string,
  cx: number,
  cy: number,
  nodes: Map<string, TreeNode>,
  config: FunctionTreeConfig,
  infos: Map<string, SubtreeInfo>
): void {
  const node = nodes.get(nodeId);
  if (!node) return;

  // Position this node centered at (cx, cy)
  node.x = cx - node.width / 2;
  node.y = cy;

  const children = node.children
    .map((cid) => nodes.get(cid))
    .filter((c): c is TreeNode => c !== undefined);

  if (children.length === 0) return;

  const childY = cy + node.height + config.nodeGapY;

  const useGrid =
    children.length > config.gridThreshold &&
    children.every((c) => c.children.length === 0);

  if (useGrid) {
    // Grid layout
    const cols = Math.ceil(Math.sqrt(children.length));
    const cellWidth = children[0].width;
    const cellHeight = children[0].height;
    const gridWidth = cols * cellWidth + (cols - 1) * config.nodeGapX;
    const gridGapY = config.nodeGapX * 0.5;
    const startX = cx - gridWidth / 2;

    for (let i = 0; i < children.length; i++) {
      const col = i % cols;
      const row = Math.floor(i / cols);
      const childCx = startX + col * (cellWidth + config.nodeGapX) + cellWidth / 2;
      const childCy = childY + row * (cellHeight + gridGapY);

      children[i].x = childCx - children[i].width / 2;
      children[i].y = childCy;
    }
  } else {
    // Row layout: distribute children horizontally, centered under parent
    const childInfos = children.map((c) => infos.get(c.id)!);
    const totalChildrenWidth =
      childInfos.reduce((sum, ci) => sum + ci.width, 0) +
      (childInfos.length - 1) * config.nodeGapX;

    let currentX = cx - totalChildrenWidth / 2;

    for (let i = 0; i < children.length; i++) {
      const child = children[i];
      const childInfo = childInfos[i];
      const childCx = currentX + childInfo.width / 2;

      assignPositions(child.id, childCx, childY, nodes, config, infos);
      currentX += childInfo.width + config.nodeGapX;
    }
  }
}

/**
 * Compute the bounding box of all nodes in the tree.
 * Returns { minX, minY, maxX, maxY } or null if empty.
 */
export function treeBounds(
  nodes: Map<string, TreeNode>
): { minX: number; minY: number; maxX: number; maxY: number } | null {
  if (nodes.size === 0) return null;

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const node of nodes.values()) {
    minX = Math.min(minX, node.x);
    minY = Math.min(minY, node.y);
    maxX = Math.max(maxX, node.x + node.width);
    maxY = Math.max(maxY, node.y + node.height);
  }

  return { minX, minY, maxX, maxY };
}

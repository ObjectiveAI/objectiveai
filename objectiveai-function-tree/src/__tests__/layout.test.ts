import { describe, it, expect } from "vitest";
import { layoutTree, treeBounds } from "../core/layout";
import { buildTree } from "../core/tree-data";
import type { TreeNode, TreeData, FunctionTreeConfig } from "../types";
import { DEFAULT_CONFIG, NODE_SIZES } from "../types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeConfig(overrides?: Partial<FunctionTreeConfig>): FunctionTreeConfig {
  return { ...DEFAULT_CONFIG, ...overrides };
}

function makeSimpleTree(childCount: number): TreeData {
  const nodes = new Map<string, TreeNode>();

  const root: TreeNode = {
    id: "root",
    kind: "function",
    label: "Root",
    parentId: null,
    children: [],
    x: 0, y: 0,
    width: NODE_SIZES.function.width,
    height: NODE_SIZES.function.height,
    state: "complete",
    data: { kind: "function", functionId: null, profileId: null, output: 0.5, taskCount: childCount, error: null },
  };
  nodes.set("root", root);

  for (let i = 0; i < childCount; i++) {
    const childId = `child-${i}`;
    const child: TreeNode = {
      id: childId,
      kind: "vector-completion",
      label: `Task ${i}`,
      parentId: "root",
      children: [],
      x: 0, y: 0,
      width: NODE_SIZES["vector-completion"].width,
      height: NODE_SIZES["vector-completion"].height,
      state: "complete",
      data: {
        kind: "vector-completion",
        taskIndex: i,
        taskPath: [i],
        scores: [0.5, 0.5],
        responses: null,
        voteCount: 5,
        votes: null,
        completions: null,
        error: null,
      },
    };
    nodes.set(childId, child);
    root.children.push(childId);
  }

  return { nodes, rootId: "root" };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("layoutTree", () => {
  it("positions single root node at origin", () => {
    const tree = makeSimpleTree(0);
    const config = makeConfig();
    layoutTree(tree, config);

    const root = tree.nodes.get("root")!;
    // Root should be centered at x=0 (x = 0 - width/2)
    expect(root.x).toBe(-NODE_SIZES.function.width / 2);
    expect(root.y).toBe(0);
  });

  it("centers parent over two children", () => {
    const tree = makeSimpleTree(2);
    const config = makeConfig();
    layoutTree(tree, config);

    const root = tree.nodes.get("root")!;
    const child0 = tree.nodes.get("child-0")!;
    const child1 = tree.nodes.get("child-1")!;

    // Children should be below root
    expect(child0.y).toBeGreaterThan(root.y);
    expect(child1.y).toBeGreaterThan(root.y);

    // Children should be at the same y level
    expect(child0.y).toBe(child1.y);

    // Root should be horizontally centered over children
    const childrenCenter = (child0.x + child0.width / 2 + child1.x + child1.width / 2) / 2;
    const rootCenter = root.x + root.width / 2;
    expect(Math.abs(rootCenter - childrenCenter)).toBeLessThan(1);
  });

  it("does not overlap siblings", () => {
    const tree = makeSimpleTree(5);
    const config = makeConfig();
    layoutTree(tree, config);

    const children = Array.from(tree.nodes.values()).filter((n) => n.id !== "root");
    children.sort((a, b) => a.x - b.x);

    for (let i = 1; i < children.length; i++) {
      const prev = children[i - 1];
      const curr = children[i];
      // No overlap: previous right edge should be before current left edge
      expect(prev.x + prev.width).toBeLessThanOrEqual(curr.x + 1); // 1px tolerance
    }
  });

  it("uses grid layout for large fan-outs", () => {
    const tree = makeSimpleTree(30); // Over default gridThreshold (20)
    const config = makeConfig({ gridThreshold: 20 });
    layoutTree(tree, config);

    const children = Array.from(tree.nodes.values()).filter((n) => n.id !== "root");

    // Grid: children should be at multiple y levels
    const uniqueYs = new Set(children.map((c) => c.y));
    expect(uniqueYs.size).toBeGreaterThan(1);
  });

  it("keeps row layout when below threshold", () => {
    const tree = makeSimpleTree(10);
    const config = makeConfig({ gridThreshold: 20 });
    layoutTree(tree, config);

    const children = Array.from(tree.nodes.values()).filter((n) => n.id !== "root");

    // Row: all children at same y level
    const uniqueYs = new Set(children.map((c) => c.y));
    expect(uniqueYs.size).toBe(1);
  });
});

describe("treeBounds", () => {
  it("returns null for empty map", () => {
    expect(treeBounds(new Map())).toBeNull();
  });

  it("computes correct bounds for laid-out tree", () => {
    const tree = makeSimpleTree(3);
    layoutTree(tree, makeConfig());

    const bounds = treeBounds(tree.nodes)!;
    expect(bounds).not.toBeNull();
    expect(bounds.minX).toBeLessThan(bounds.maxX);
    expect(bounds.minY).toBeLessThan(bounds.maxY);

    // All nodes should be within bounds
    for (const node of tree.nodes.values()) {
      expect(node.x).toBeGreaterThanOrEqual(bounds.minX);
      expect(node.y).toBeGreaterThanOrEqual(bounds.minY);
      expect(node.x + node.width).toBeLessThanOrEqual(bounds.maxX);
      expect(node.y + node.height).toBeLessThanOrEqual(bounds.maxY);
    }
  });
});

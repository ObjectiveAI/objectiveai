import { describe, it, expect } from "vitest";
import { AnimationController } from "../core/animation";
import type { TreeNode } from "../types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeNode(
  id: string,
  x: number,
  y: number,
  parentId: string | null = null
): TreeNode {
  return {
    id,
    kind: "function",
    label: id,
    parentId,
    children: [],
    x,
    y,
    width: 200,
    height: 80,
    state: "complete",
    data: { taskCount: 0 },
  } as TreeNode;
}

function toMap(...nodes: TreeNode[]): Map<string, TreeNode> {
  return new Map(nodes.map((n) => [n.id, n]));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("AnimationController", () => {
  describe("scheduleTransitions", () => {
    it("first render: all nodes get enter transitions", () => {
      const ctrl = new AnimationController();
      const nodes = toMap(makeNode("a", 100, 200), makeNode("b", 300, 400));
      ctrl.scheduleTransitions(null, nodes, 300, 1000);

      const a = ctrl.getInterpolated("a", 1000);
      const b = ctrl.getInterpolated("b", 1000);
      expect(a).not.toBeNull();
      expect(b).not.toBeNull();
      // At t=0 opacity should be 0 (entering)
      expect(a!.opacity).toBe(0);
      expect(b!.opacity).toBe(0);
    });

    it("enter transitions start from parent position when parent exists", () => {
      const ctrl = new AnimationController();
      const parent = makeNode("p", 100, 50);
      const child = makeNode("c", 200, 200, "p");
      parent.children = ["c"];

      const prev = toMap(parent);
      const next = toMap(parent, child);
      ctrl.scheduleTransitions(prev, next, 300, 1000);

      const c = ctrl.getInterpolated("c", 1000);
      expect(c).not.toBeNull();
      // Should start from parent's center-bottom, not child's own position
      expect(c!.x).not.toBe(200);
      expect(c!.opacity).toBe(0);
    });

    it("enter at own position when no parent exists", () => {
      const ctrl = new AnimationController();
      const node = makeNode("orphan", 150, 250);

      const prev = new Map<string, TreeNode>();
      const next = toMap(node);
      ctrl.scheduleTransitions(prev, next, 300, 1000);

      const o = ctrl.getInterpolated("orphan", 1000);
      expect(o).not.toBeNull();
      // At t=0, should be at own position but invisible
      expect(o!.x).toBe(150);
      expect(o!.y).toBe(250);
      expect(o!.opacity).toBe(0);
    });

    it("update transitions for moved nodes", () => {
      const ctrl = new AnimationController();
      const before = makeNode("a", 100, 200);
      const after = makeNode("a", 300, 400);

      ctrl.scheduleTransitions(toMap(before), toMap(after), 300, 1000);

      const a = ctrl.getInterpolated("a", 1000);
      expect(a).not.toBeNull();
      // At t=0, should be at old position
      expect(a!.x).toBe(100);
      expect(a!.y).toBe(200);
      // Opacity stays 1 for updates
      expect(a!.opacity).toBe(1);
    });

    it("no transition for unmoved nodes", () => {
      const ctrl = new AnimationController();
      const node = makeNode("a", 100, 200);

      ctrl.scheduleTransitions(toMap(node), toMap(node), 300, 1000);

      // No movement = no transition
      expect(ctrl.getInterpolated("a", 1000)).toBeNull();
    });

    it("exit transitions for removed nodes", () => {
      const ctrl = new AnimationController();
      const node = makeNode("gone", 100, 200);

      ctrl.scheduleTransitions(toMap(node), new Map(), 300, 1000);

      const g = ctrl.getInterpolated("gone", 1000);
      expect(g).not.toBeNull();
      // At t=0, visible at old position
      expect(g!.x).toBe(100);
      expect(g!.y).toBe(200);
      expect(g!.opacity).toBe(1);
    });
  });

  describe("getInterpolated", () => {
    it("returns null for unknown node ID", () => {
      const ctrl = new AnimationController();
      expect(ctrl.getInterpolated("nonexistent", 1000)).toBeNull();
    });

    it("at t=0 returns from values", () => {
      const ctrl = new AnimationController();
      const before = makeNode("a", 0, 0);
      const after = makeNode("a", 100, 200);

      ctrl.scheduleTransitions(toMap(before), toMap(after), 300, 1000);

      const a = ctrl.getInterpolated("a", 1000);
      expect(a!.x).toBe(0);
      expect(a!.y).toBe(0);
      expect(a!.opacity).toBe(1);
    });

    it("at t=duration returns to values", () => {
      const ctrl = new AnimationController();
      const before = makeNode("a", 0, 0);
      const after = makeNode("a", 100, 200);

      ctrl.scheduleTransitions(toMap(before), toMap(after), 300, 1000);

      const a = ctrl.getInterpolated("a", 1300);
      expect(a!.x).toBe(100);
      expect(a!.y).toBe(200);
      expect(a!.opacity).toBe(1);
    });

    it("at midpoint uses easeOutCubic interpolation", () => {
      const ctrl = new AnimationController();
      const before = makeNode("a", 0, 0);
      const after = makeNode("a", 100, 0);

      ctrl.scheduleTransitions(toMap(before), toMap(after), 300, 1000);

      // easeOutCubic(0.5) = 1 - (1 - 0.5)^3 = 1 - 0.125 = 0.875
      const a = ctrl.getInterpolated("a", 1150); // t=0.5
      expect(a!.x).toBeCloseTo(87.5, 1);
    });

    it("exit node fades to opacity 0", () => {
      const ctrl = new AnimationController();
      const node = makeNode("a", 50, 50);

      ctrl.scheduleTransitions(toMap(node), new Map(), 300, 1000);

      // At end of animation, fully transparent
      const a = ctrl.getInterpolated("a", 1300);
      expect(a!.opacity).toBe(0);
    });
  });

  describe("isAnimating", () => {
    it("true during animation, false after", () => {
      const ctrl = new AnimationController();
      const before = makeNode("a", 0, 0);
      const after = makeNode("a", 100, 100);

      ctrl.scheduleTransitions(toMap(before), toMap(after), 100, 1000);

      expect(ctrl.isAnimating(1050)).toBe(true);
      expect(ctrl.isAnimating(1100)).toBe(false);
    });
  });

  describe("reset", () => {
    it("clears all transitions", () => {
      const ctrl = new AnimationController();
      const before = makeNode("a", 0, 0);
      const after = makeNode("a", 100, 100);

      ctrl.scheduleTransitions(toMap(before), toMap(after), 300, 1000);
      expect(ctrl.isAnimating(1000)).toBe(true);

      ctrl.reset();
      expect(ctrl.isAnimating(1000)).toBe(false);
      expect(ctrl.getInterpolated("a", 1000)).toBeNull();
    });
  });
});

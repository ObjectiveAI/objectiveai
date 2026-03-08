import { describe, it, expect, vi } from "vitest";
import { InteractionHandler } from "../core/interaction";
import { Viewport } from "../core/viewport";
import type { TreeNode } from "../types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeNode(
  id: string,
  x: number,
  y: number,
  w = 200,
  h = 80
): TreeNode {
  return {
    id,
    kind: "function",
    label: id,
    parentId: null,
    children: [],
    x,
    y,
    width: w,
    height: h,
    state: "complete",
    data: { taskCount: 0 },
  } as TreeNode;
}

function toMap(...nodes: TreeNode[]): Map<string, TreeNode> {
  return new Map(nodes.map((n) => [n.id, n]));
}

function createMockCanvas() {
  return {
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    getBoundingClientRect: vi.fn(() => ({
      left: 0,
      top: 0,
      width: 800,
      height: 600,
    })),
    style: { cursor: "" },
  } as unknown as HTMLCanvasElement;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("InteractionHandler", () => {
  describe("hitTest", () => {
    it("returns null with empty node map", () => {
      const canvas = createMockCanvas();
      const vp = new Viewport(0.02, 3);
      const handler = new InteractionHandler(canvas, vp, {});

      expect(handler.hitTest(100, 100)).toBeNull();
    });

    it("returns node when coords fall within bounds", () => {
      const canvas = createMockCanvas();
      const vp = new Viewport(0.02, 3);
      const handler = new InteractionHandler(canvas, vp, {});

      const node = makeNode("a", 100, 100, 200, 80);
      handler.setNodes(toMap(node));

      // Click inside the node (viewport is identity: zoom=1, pan=0)
      const hit = handler.hitTest(150, 130);
      expect(hit).not.toBeNull();
      expect(hit!.id).toBe("a");
    });

    it("returns null when clicking empty space", () => {
      const canvas = createMockCanvas();
      const vp = new Viewport(0.02, 3);
      const handler = new InteractionHandler(canvas, vp, {});

      const node = makeNode("a", 100, 100, 200, 80);
      handler.setNodes(toMap(node));

      // Click outside the node
      expect(handler.hitTest(50, 50)).toBeNull();
      expect(handler.hitTest(350, 200)).toBeNull();
    });

    it("respects viewport pan offset", () => {
      const canvas = createMockCanvas();
      const vp = new Viewport(0.02, 3);
      const handler = new InteractionHandler(canvas, vp, {});

      const node = makeNode("a", 100, 100, 200, 80);
      handler.setNodes(toMap(node));

      // At identity, screen(150,130) hits the node
      expect(handler.hitTest(150, 130)).not.toBeNull();

      // pan(100,100) at zoom=1: panX -= 100, panY -= 100 → panX = -100
      // screenToWorld(150,130) = (150 + (-100), 130 + (-100)) = (50, 30) → misses
      vp.pan(100, 100);
      expect(handler.hitTest(150, 130)).toBeNull();

      // screenToWorld(200,200) = (200 + (-100), 200 + (-100)) = (100, 100) → hits
      const hit = handler.hitTest(200, 200);
      expect(hit).not.toBeNull();
      expect(hit!.id).toBe("a");
    });

    it("respects viewport zoom", () => {
      const canvas = createMockCanvas();
      const vp = new Viewport(0.02, 3);
      const handler = new InteractionHandler(canvas, vp, {});

      // Node at world (0, 0) with size 100x50
      const node = makeNode("a", 0, 0, 100, 50);
      handler.setNodes(toMap(node));

      // At zoom=1, node spans screen 0-100, 0-50
      expect(handler.hitTest(50, 25)).not.toBeNull();
      expect(handler.hitTest(150, 25)).toBeNull();

      // Zoom to 2x centered at origin — node now spans screen 0-200, 0-100
      vp.zoomAt(1, 0, 0);
      expect(handler.hitTest(150, 50)).not.toBeNull();
    });

    it("returns topmost (last) node on overlap", () => {
      const canvas = createMockCanvas();
      const vp = new Viewport(0.02, 3);
      const handler = new InteractionHandler(canvas, vp, {});

      // Two nodes at the same position
      const a = makeNode("a", 100, 100, 200, 80);
      const b = makeNode("b", 100, 100, 200, 80);
      handler.setNodes(toMap(a, b));

      // Should return the last one (b) since it's drawn on top
      const hit = handler.hitTest(150, 130);
      expect(hit).not.toBeNull();
      expect(hit!.id).toBe("b");
    });

    it("setNodes updates hit targets", () => {
      const canvas = createMockCanvas();
      const vp = new Viewport(0.02, 3);
      const handler = new InteractionHandler(canvas, vp, {});

      const a = makeNode("a", 100, 100, 200, 80);
      handler.setNodes(toMap(a));
      expect(handler.hitTest(150, 130)?.id).toBe("a");

      // Replace with different node
      const b = makeNode("b", 500, 500, 200, 80);
      handler.setNodes(toMap(b));
      expect(handler.hitTest(150, 130)).toBeNull();
      expect(handler.hitTest(550, 530)?.id).toBe("b");
    });
  });

  describe("lifecycle", () => {
    it("constructor registers 9 event listeners", () => {
      const canvas = createMockCanvas();
      const vp = new Viewport(0.02, 3);
      new InteractionHandler(canvas, vp, {});

      expect(canvas.addEventListener).toHaveBeenCalledTimes(9);
    });

    it("destroy removes all 9 event listeners", () => {
      const canvas = createMockCanvas();
      const vp = new Viewport(0.02, 3);
      const handler = new InteractionHandler(canvas, vp, {});

      handler.destroy();
      expect(canvas.removeEventListener).toHaveBeenCalledTimes(9);
    });
  });
});

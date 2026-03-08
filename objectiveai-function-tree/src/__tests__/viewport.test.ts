import { describe, it, expect } from "vitest";
import { Viewport } from "../core/viewport";

describe("Viewport", () => {
  describe("coordinate transforms", () => {
    it("identity transform (zoom=1, pan=0,0)", () => {
      const vp = new Viewport();
      const world = vp.screenToWorld(100, 200);
      expect(world.x).toBe(100);
      expect(world.y).toBe(200);

      const screen = vp.worldToScreen(100, 200);
      expect(screen.x).toBe(100);
      expect(screen.y).toBe(200);
    });

    it("screen-to-world-to-screen roundtrip", () => {
      const vp = new Viewport();
      vp.panX = 50;
      vp.panY = -30;
      vp.zoom = 1.5;

      const screen = { x: 200, y: 150 };
      const world = vp.screenToWorld(screen.x, screen.y);
      const back = vp.worldToScreen(world.x, world.y);

      expect(Math.abs(back.x - screen.x)).toBeLessThan(0.001);
      expect(Math.abs(back.y - screen.y)).toBeLessThan(0.001);
    });

    it("panning shifts world coordinates", () => {
      const vp = new Viewport();
      vp.panX = 100;
      vp.panY = 50;

      const world = vp.screenToWorld(0, 0);
      expect(world.x).toBe(100); // screen 0 maps to world panX
      expect(world.y).toBe(50);
    });

    it("zooming scales world coordinates", () => {
      const vp = new Viewport();
      vp.zoom = 2;

      const world = vp.screenToWorld(200, 100);
      expect(world.x).toBe(100); // 200 / 2
      expect(world.y).toBe(50); // 100 / 2
    });
  });

  describe("isVisible", () => {
    it("returns true for rect in viewport", () => {
      const vp = new Viewport();
      expect(vp.isVisible(10, 10, 50, 50, 800, 600)).toBe(true);
    });

    it("returns false for rect far off-screen", () => {
      const vp = new Viewport();
      expect(vp.isVisible(2000, 2000, 50, 50, 800, 600)).toBe(false);
    });

    it("returns true for partially visible rect", () => {
      const vp = new Viewport();
      // Rect starts at -30, so its right edge is at 20, which is visible
      expect(vp.isVisible(-30, 0, 50, 50, 800, 600)).toBe(true);
    });

    it("accounts for zoom", () => {
      const vp = new Viewport();
      vp.zoom = 0.5;
      // At zoom 0.5, world x=1700 maps to screen x=850, which is > 800
      expect(vp.isVisible(1700, 0, 50, 50, 800, 600)).toBe(false);
    });
  });

  describe("pan", () => {
    it("adjusts pan by screen delta", () => {
      const vp = new Viewport();
      vp.pan(100, 50);

      // Pan moves in opposite direction of drag (drag right = pan left)
      expect(vp.panX).toBe(-100);
      expect(vp.panY).toBe(-50);
    });

    it("accounts for zoom when panning", () => {
      const vp = new Viewport();
      vp.zoom = 2;
      vp.pan(100, 50);

      // At zoom 2, 100 screen pixels = 50 world units
      expect(vp.panX).toBe(-50);
      expect(vp.panY).toBe(-25);
    });
  });

  describe("zoomAt", () => {
    it("clamps zoom to min/max", () => {
      const vp = new Viewport(0.1, 5);

      // Zoom way in
      for (let i = 0; i < 100; i++) {
        vp.zoomAt(0.5, 400, 300);
      }
      expect(vp.zoom).toBeLessThanOrEqual(5);

      // Zoom way out
      for (let i = 0; i < 100; i++) {
        vp.zoomAt(-0.5, 400, 300);
      }
      expect(vp.zoom).toBeGreaterThanOrEqual(0.1);
    });
  });

  describe("fitToContent", () => {
    it("fits nodes into viewport", () => {
      const vp = new Viewport();

      const nodes = new Map([
        [
          "a",
          {
            id: "a", kind: "function" as const, label: "A",
            parentId: null, children: [],
            x: 0, y: 0, width: 200, height: 80,
            state: "complete" as const,
            data: { kind: "function" as const, functionId: null, profileId: null, output: null, taskCount: 0, error: null },
          },
        ],
        [
          "b",
          {
            id: "b", kind: "vector-completion" as const, label: "B",
            parentId: "a", children: [],
            x: 500, y: 200, width: 180, height: 70,
            state: "complete" as const,
            data: {
              kind: "vector-completion" as const, taskIndex: 0, taskPath: [0],
              scores: null, responses: null, voteCount: 0, votes: null,
              completions: null, error: null,
            },
          },
        ],
      ]);

      vp.fitToContent(nodes, 800, 600);

      expect(vp.zoom).toBeGreaterThan(0);
      expect(vp.zoom).toBeLessThanOrEqual(3);
    });

    it("enforces minimum zoom floor", () => {
      const vp = new Viewport();

      // Nodes spread very far apart — natural zoom would be very low
      const nodes = new Map([
        [
          "a",
          {
            id: "a", kind: "function" as const, label: "A",
            parentId: null, children: [],
            x: 0, y: 0, width: 200, height: 80,
            state: "complete" as const,
            data: { kind: "function" as const, functionId: null, profileId: null, output: null, taskCount: 0, error: null },
          },
        ],
        [
          "b",
          {
            id: "b", kind: "vector-completion" as const, label: "B",
            parentId: "a", children: [],
            x: 5000, y: 0, width: 180, height: 70,
            state: "complete" as const,
            data: {
              kind: "vector-completion" as const, taskIndex: 0, taskPath: [0],
              scores: null, responses: null, voteCount: 0, votes: null,
              completions: null, error: null,
            },
          },
        ],
      ]);

      vp.fitToContent(nodes, 800, 600, 40);

      // Should use natural zoom to fit all content, not clamp up
      expect(vp.zoom).toBeLessThan(0.4);
      expect(vp.zoom).toBeGreaterThan(0);
    });
  });

  describe("snapshot", () => {
    it("captures current state", () => {
      const vp = new Viewport();
      vp.panX = 10;
      vp.panY = 20;
      vp.zoom = 1.5;

      const snap = vp.snapshot();
      expect(snap).toEqual({ panX: 10, panY: 20, zoom: 1.5 });
    });
  });
});

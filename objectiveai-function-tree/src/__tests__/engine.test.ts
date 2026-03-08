import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { FunctionTreeEngine } from "../core/engine";
import { createMockCanvas } from "./helpers/mock-canvas";
import type { InputFunctionExecution } from "../types";

// ---------------------------------------------------------------------------
// Globals mocking
// ---------------------------------------------------------------------------

let rafCallbacks: Map<number, FrameRequestCallback>;
let rafId: number;

beforeEach(() => {
  rafCallbacks = new Map();
  rafId = 0;

  vi.stubGlobal(
    "requestAnimationFrame",
    vi.fn((cb: FrameRequestCallback) => {
      const id = ++rafId;
      rafCallbacks.set(id, cb);
      return id;
    })
  );
  vi.stubGlobal(
    "cancelAnimationFrame",
    vi.fn((id: number) => {
      rafCallbacks.delete(id);
    })
  );
  vi.stubGlobal("devicePixelRatio", 1);

  // performance.now is already available in Node
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeExecution(
  scores: number[] = [0.7, 0.3]
): InputFunctionExecution {
  return {
    id: "exec-1",
    output: scores[0],
    function: "test/func",
    tasks: [
      {
        index: 0,
        task_index: 0,
        task_path: [0],
        scores,
        votes: [
          {
            model: "model-a-xxxxxxxxxx1234",
            vote: scores,
            weight: 1,
            from_cache: false,
            from_rng: false,
          },
        ],
        completions: [],
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("FunctionTreeEngine", () => {
  it("constructor succeeds with valid canvas", () => {
    const { canvas } = createMockCanvas();
    const engine = new FunctionTreeEngine(canvas);
    expect(engine).toBeDefined();
    expect((canvas as any).style.cursor).toBe("grab");
    engine.destroy();
  });

  it("constructor throws if getContext returns null", () => {
    const { canvas } = createMockCanvas();
    (canvas.getContext as any).mockReturnValueOnce(null);

    expect(() => new FunctionTreeEngine(canvas)).toThrow(
      "Failed to get 2D rendering context"
    );
  });

  it("setData(null) clears tree without crash", () => {
    const { canvas } = createMockCanvas();
    const engine = new FunctionTreeEngine(canvas);

    engine.setData(null);
    expect(engine.getSelectedNode()).toBeNull();
    engine.destroy();
  });

  it("setData with valid execution builds tree", () => {
    const { canvas } = createMockCanvas();
    const engine = new FunctionTreeEngine(canvas);

    engine.setData(makeExecution());
    expect(engine.getSelectedNode()).toBeNull(); // No selection yet
    engine.destroy();
  });

  it("setConfig updates without crash", () => {
    const { canvas } = createMockCanvas();
    const engine = new FunctionTreeEngine(canvas);

    engine.setConfig({ theme: "dark" });
    engine.setConfig({ theme: "light" });
    engine.setConfig({ nodeGapX: 48 });
    engine.destroy();
  });

  it("deselect fires onSelectedNodeChange with null", () => {
    const { canvas } = createMockCanvas();
    const engine = new FunctionTreeEngine(canvas);
    const cb = vi.fn();
    engine.onSelectedNodeChange = cb;

    engine.deselect();
    expect(cb).toHaveBeenCalledWith(null);
    engine.destroy();
  });

  it("destroy cancels RAF", () => {
    const { canvas } = createMockCanvas();
    const engine = new FunctionTreeEngine(canvas);

    // Trigger a render to start the loop
    engine.setData(makeExecution());

    engine.destroy();
    expect(cancelAnimationFrame).toHaveBeenCalled();
  });

  it("destroy prevents further setData from having effect", () => {
    const { canvas } = createMockCanvas();
    const engine = new FunctionTreeEngine(canvas);

    engine.destroy();

    // Should not throw
    engine.setData(makeExecution());
    expect(engine.getSelectedNode()).toBeNull();
  });

  it("fitToContent does not crash on empty tree", () => {
    const { canvas } = createMockCanvas();
    const engine = new FunctionTreeEngine(canvas);

    // No data set — should be a no-op
    engine.fitToContent();
    engine.destroy();
  });

  it("zoomIn and zoomOut do not crash", () => {
    const { canvas } = createMockCanvas();
    const engine = new FunctionTreeEngine(canvas);

    engine.setData(makeExecution());
    engine.zoomIn();
    engine.zoomOut();
    engine.destroy();
  });
});

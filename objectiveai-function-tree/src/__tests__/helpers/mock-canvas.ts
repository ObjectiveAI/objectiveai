import { vi } from "vitest";

/**
 * Creates a minimal mock HTMLCanvasElement with a stub 2D context.
 * Suitable for unit testing engine and interaction logic without a real browser.
 */
export function createMockCanvas(width = 800, height = 600) {
  const listeners = new Map<string, Function[]>();

  const ctx = {
    beginPath: vi.fn(),
    closePath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    bezierCurveTo: vi.fn(),
    arc: vi.fn(),
    fill: vi.fn(),
    stroke: vi.fn(),
    fillRect: vi.fn(),
    clearRect: vi.fn(),
    fillText: vi.fn(),
    measureText: vi.fn(() => ({ width: 50 })),
    resetTransform: vi.fn(),
    setTransform: vi.fn(),
    transform: vi.fn(),
    scale: vi.fn(),
    save: vi.fn(),
    restore: vi.fn(),
    clip: vi.fn(),
    roundRect: vi.fn(),
    createLinearGradient: vi.fn(() => ({
      addColorStop: vi.fn(),
    })),
    fillStyle: "",
    strokeStyle: "",
    lineWidth: 1,
    font: "",
    textAlign: "left" as CanvasTextAlign,
    textBaseline: "top" as CanvasTextBaseline,
    globalAlpha: 1,
  };

  const canvas = {
    getContext: vi.fn(() => ctx),
    getBoundingClientRect: vi.fn(() => ({
      left: 0,
      top: 0,
      width,
      height,
      right: width,
      bottom: height,
    })),
    addEventListener: vi.fn((event: string, handler: Function) => {
      const handlers = listeners.get(event) || [];
      handlers.push(handler);
      listeners.set(event, handlers);
    }),
    removeEventListener: vi.fn((event: string, handler: Function) => {
      const handlers = listeners.get(event) || [];
      listeners.set(
        event,
        handlers.filter((h) => h !== handler)
      );
    }),
    width: width * 2,
    height: height * 2,
    clientWidth: width,
    clientHeight: height,
    style: { cursor: "", width: "", height: "" },
  } as unknown as HTMLCanvasElement;

  return { canvas, ctx, listeners };
}

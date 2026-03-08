import type { TreeNode } from "../types";

// ---------------------------------------------------------------------------
// Viewport: camera transform (pan + zoom) for the canvas
// ---------------------------------------------------------------------------

export interface ViewportState {
  panX: number;
  panY: number;
  zoom: number;
}

export class Viewport {
  panX = 0;
  panY = 0;
  zoom = 1;

  private _animating = false;
  private _animStart = 0;
  private _animDuration = 0;
  private _animFrom: ViewportState = { panX: 0, panY: 0, zoom: 1 };
  private _animTo: ViewportState = { panX: 0, panY: 0, zoom: 1 };

  constructor(
    private minZoom: number = 0.02,
    private maxZoom: number = 3
  ) {}

  /** Convert screen (canvas pixel) coordinates to world coordinates. */
  screenToWorld(sx: number, sy: number): { x: number; y: number } {
    return {
      x: sx / this.zoom + this.panX,
      y: sy / this.zoom + this.panY,
    };
  }

  /** Convert world coordinates to screen (canvas pixel) coordinates. */
  worldToScreen(wx: number, wy: number): { x: number; y: number } {
    return {
      x: (wx - this.panX) * this.zoom,
      y: (wy - this.panY) * this.zoom,
    };
  }

  /** Check if a world-space rectangle is visible in the viewport. */
  isVisible(
    x: number,
    y: number,
    w: number,
    h: number,
    canvasWidth: number,
    canvasHeight: number
  ): boolean {
    const screen = this.worldToScreen(x, y);
    const sw = w * this.zoom;
    const sh = h * this.zoom;

    return (
      screen.x + sw > 0 &&
      screen.x < canvasWidth &&
      screen.y + sh > 0 &&
      screen.y < canvasHeight
    );
  }

  /** Apply the viewport transform to a canvas context (multiplies with existing transform). */
  applyTransform(ctx: CanvasRenderingContext2D): void {
    ctx.transform(this.zoom, 0, 0, this.zoom, -this.panX * this.zoom, -this.panY * this.zoom);
  }

  /** Zoom by a delta, centering on a screen point. */
  zoomAt(delta: number, screenX: number, screenY: number): void {
    const worldBefore = this.screenToWorld(screenX, screenY);
    this.zoom = clamp(this.zoom * (1 + delta), this.minZoom, this.maxZoom);
    const worldAfter = this.screenToWorld(screenX, screenY);

    this.panX += worldBefore.x - worldAfter.x;
    this.panY += worldBefore.y - worldAfter.y;
  }

  /** Pan by screen-space delta. */
  pan(dx: number, dy: number): void {
    this.panX -= dx / this.zoom;
    this.panY -= dy / this.zoom;
  }

  /** Zoom to fit all nodes in the viewport with padding. */
  fitToContent(
    nodes: Map<string, TreeNode>,
    canvasWidth: number,
    canvasHeight: number,
    padding: number = 40
  ): void {
    if (nodes.size === 0) return;

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

    const contentWidth = maxX - minX;
    const contentHeight = maxY - minY;

    if (contentWidth <= 0 || contentHeight <= 0) return;

    const availableWidth = canvasWidth - padding * 2;
    const availableHeight = canvasHeight - padding * 2;

    const naturalZoom = Math.min(
      availableWidth / contentWidth,
      availableHeight / contentHeight
    );

    this.zoom = clamp(naturalZoom, this.minZoom, this.maxZoom);

    // Center content
    this.panX = minX - (canvasWidth / this.zoom - contentWidth) / 2;
    this.panY = minY - (canvasHeight / this.zoom - contentHeight) / 2;
  }

  /** Animate to a target viewport state. */
  animateTo(
    target: Partial<ViewportState>,
    duration: number = 300
  ): void {
    this._animFrom = { panX: this.panX, panY: this.panY, zoom: this.zoom };
    this._animTo = {
      panX: target.panX ?? this.panX,
      panY: target.panY ?? this.panY,
      zoom: target.zoom ?? this.zoom,
    };
    this._animStart = performance.now();
    this._animDuration = duration;
    this._animating = true;
  }

  /** Tick the animation. Returns true if still animating. */
  tick(now: number): boolean {
    if (!this._animating) return false;

    const elapsed = now - this._animStart;
    const t = Math.min(elapsed / this._animDuration, 1);
    const eased = easeOutCubic(t);

    this.panX = lerp(this._animFrom.panX, this._animTo.panX, eased);
    this.panY = lerp(this._animFrom.panY, this._animTo.panY, eased);
    this.zoom = lerp(this._animFrom.zoom, this._animTo.zoom, eased);

    if (t >= 1) {
      this._animating = false;
    }

    return this._animating;
  }

  get isAnimating(): boolean {
    return this._animating;
  }

  /** Save current state for later comparison. */
  snapshot(): ViewportState {
    return { panX: this.panX, panY: this.panY, zoom: this.zoom };
  }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

function easeOutCubic(t: number): number {
  return 1 - Math.pow(1 - t, 3);
}

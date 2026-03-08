import type { TreeNode } from "../types";
import type { Viewport } from "./viewport";

// ---------------------------------------------------------------------------
// Interaction handler: mouse/touch events → viewport changes + hit testing
// ---------------------------------------------------------------------------

export interface InteractionCallbacks {
  onNodeClick?: (node: TreeNode) => void;
  onNodeHover?: (node: TreeNode | null) => void;
  onViewportChange?: () => void;
}

export class InteractionHandler {
  private _isDragging = false;
  private _lastX = 0;
  private _lastY = 0;
  private _hoveredNodeId: string | null = null;

  // Touch state
  private _touches: Map<number, { x: number; y: number }> = new Map();
  private _lastPinchDist = 0;

  constructor(
    private canvas: HTMLCanvasElement,
    private viewport: Viewport,
    private callbacks: InteractionCallbacks
  ) {
    this._onMouseDown = this._onMouseDown.bind(this);
    this._onMouseMove = this._onMouseMove.bind(this);
    this._onMouseUp = this._onMouseUp.bind(this);
    this._onWheel = this._onWheel.bind(this);
    this._onDblClick = this._onDblClick.bind(this);
    this._onTouchStart = this._onTouchStart.bind(this);
    this._onTouchMove = this._onTouchMove.bind(this);
    this._onTouchEnd = this._onTouchEnd.bind(this);

    canvas.addEventListener("mousedown", this._onMouseDown);
    canvas.addEventListener("mousemove", this._onMouseMove);
    canvas.addEventListener("mouseup", this._onMouseUp);
    canvas.addEventListener("mouseleave", this._onMouseUp);
    canvas.addEventListener("wheel", this._onWheel, { passive: false });
    canvas.addEventListener("dblclick", this._onDblClick);
    canvas.addEventListener("touchstart", this._onTouchStart);
    canvas.addEventListener("touchmove", this._onTouchMove);
    canvas.addEventListener("touchend", this._onTouchEnd);
  }

  /** Currently active nodes for hit testing. Set by the engine before rendering. */
  private _nodes: Map<string, TreeNode> = new Map();

  setNodes(nodes: Map<string, TreeNode>): void {
    this._nodes = nodes;
  }

  /** Hit test: find the topmost node at screen coordinates. */
  hitTest(screenX: number, screenY: number): TreeNode | null {
    const world = this.viewport.screenToWorld(screenX, screenY);

    // Reverse iteration for z-order (later nodes drawn on top)
    const entries = Array.from(this._nodes.values()).reverse();
    for (const node of entries) {
      if (
        world.x >= node.x &&
        world.x <= node.x + node.width &&
        world.y >= node.y &&
        world.y <= node.y + node.height
      ) {
        return node;
      }
    }
    return null;
  }

  // -- Mouse handlers -------------------------------------------------------

  private _onMouseDown(e: MouseEvent): void {
    if (e.button !== 0) return;
    const rect = this.canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;

    const hit = this.hitTest(sx, sy);
    if (hit) {
      // Node click — don't start drag
      this.callbacks.onNodeClick?.(hit);
      return;
    }

    // Start pan drag
    this._isDragging = true;
    this._lastX = e.clientX;
    this._lastY = e.clientY;
    this.canvas.style.cursor = "grabbing";
  }

  private _onMouseMove(e: MouseEvent): void {
    const rect = this.canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;

    if (this._isDragging) {
      const dx = e.clientX - this._lastX;
      const dy = e.clientY - this._lastY;
      this._lastX = e.clientX;
      this._lastY = e.clientY;
      this.viewport.pan(dx, dy);
      this.callbacks.onViewportChange?.();
      return;
    }

    // Hover detection
    const hit = this.hitTest(sx, sy);
    const hitId = hit?.id ?? null;
    if (hitId !== this._hoveredNodeId) {
      this._hoveredNodeId = hitId;
      this.canvas.style.cursor = hit ? "pointer" : "grab";
      this.callbacks.onNodeHover?.(hit);
    }
  }

  private _onMouseUp(_e: MouseEvent): void {
    if (this._isDragging) {
      this._isDragging = false;
      this.canvas.style.cursor = "grab";
    }
  }

  private _onWheel(e: WheelEvent): void {
    // Only intercept wheel when Ctrl/Meta is held (pinch-to-zoom gesture).
    // Otherwise let the page scroll normally.
    if (!e.ctrlKey && !e.metaKey) return;
    e.preventDefault();
    const rect = this.canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    const delta = -e.deltaY * 0.001;
    this.viewport.zoomAt(delta, sx, sy);
    this.callbacks.onViewportChange?.();
  }

  private _onDblClick(e: MouseEvent): void {
    const rect = this.canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    const hit = this.hitTest(sx, sy);

    if (hit) {
      // Zoom to fit this node's subtree
      // For now, just zoom to the node itself
      this.viewport.animateTo({
        panX: hit.x + hit.width / 2 - this.canvas.width / (2 * 1.5),
        panY: hit.y - 20,
        zoom: 1.5,
      });
      this.callbacks.onViewportChange?.();
    }
  }

  // -- Touch handlers -------------------------------------------------------

  private _onTouchStart(e: TouchEvent): void {
    for (let i = 0; i < e.changedTouches.length; i++) {
      const touch = e.changedTouches[i];
      this._touches.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
    }

    if (this._touches.size === 1) {
      const t = Array.from(this._touches.values())[0];
      this._lastX = t.x;
      this._lastY = t.y;
      // Don't start drag yet — wait for touchmove to confirm intent
      this._isDragging = false;
    } else if (this._touches.size === 2) {
      // Pinch zoom — prevent page scroll
      e.preventDefault();
      this._isDragging = false;
      const pts = Array.from(this._touches.values());
      this._lastPinchDist = Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y);
    }
  }

  private _onTouchMove(e: TouchEvent): void {
    for (let i = 0; i < e.changedTouches.length; i++) {
      const touch = e.changedTouches[i];
      this._touches.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
    }

    if (this._touches.size === 1 && this._isDragging) {
      const t = Array.from(this._touches.values())[0];
      const dx = t.x - this._lastX;
      const dy = t.y - this._lastY;
      this._lastX = t.x;
      this._lastY = t.y;
      this.viewport.pan(dx, dy);
      this.callbacks.onViewportChange?.();
    } else if (this._touches.size === 2) {
      const pts = Array.from(this._touches.values());
      const dist = Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y);
      if (this._lastPinchDist > 0) {
        const delta = (dist - this._lastPinchDist) * 0.005;
        const cx = (pts[0].x + pts[1].x) / 2;
        const cy = (pts[0].y + pts[1].y) / 2;
        const rect = this.canvas.getBoundingClientRect();
        this.viewport.zoomAt(delta, cx - rect.left, cy - rect.top);
        this.callbacks.onViewportChange?.();
      }
      this._lastPinchDist = dist;
    }
  }

  private _onTouchEnd(e: TouchEvent): void {
    for (let i = 0; i < e.changedTouches.length; i++) {
      this._touches.delete(e.changedTouches[i].identifier);
    }

    if (this._touches.size === 0) {
      this._isDragging = false;
    } else if (this._touches.size === 1) {
      const t = Array.from(this._touches.values())[0];
      this._lastX = t.x;
      this._lastY = t.y;
      this._isDragging = true;
    }
  }

  // -- Cleanup --------------------------------------------------------------

  destroy(): void {
    this.canvas.removeEventListener("mousedown", this._onMouseDown);
    this.canvas.removeEventListener("mousemove", this._onMouseMove);
    this.canvas.removeEventListener("mouseup", this._onMouseUp);
    this.canvas.removeEventListener("mouseleave", this._onMouseUp);
    this.canvas.removeEventListener("wheel", this._onWheel);
    this.canvas.removeEventListener("dblclick", this._onDblClick);
    this.canvas.removeEventListener("touchstart", this._onTouchStart);
    this.canvas.removeEventListener("touchmove", this._onTouchMove);
    this.canvas.removeEventListener("touchend", this._onTouchEnd);
  }
}

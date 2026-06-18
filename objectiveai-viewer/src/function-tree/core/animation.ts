import type { TreeNode } from "../types";

// ---------------------------------------------------------------------------
// Animation controller for smooth tree transitions
// ---------------------------------------------------------------------------

export interface NodeTransition {
  nodeId: string;
  type: "enter" | "update" | "exit";
  startTime: number;
  duration: number;
  fromX: number;
  fromY: number;
  fromOpacity: number;
  toX: number;
  toY: number;
  toOpacity: number;
}

export interface InterpolatedState {
  x: number;
  y: number;
  opacity: number;
}

export class AnimationController {
  private transitions = new Map<string, NodeTransition>();

  /**
   * Schedule transitions by diffing previous and next tree states.
   * - New nodes: enter (fade in from parent position)
   * - Moved nodes: update (slide to new position)
   * - Removed nodes: exit (fade out)
   */
  scheduleTransitions(
    prevNodes: Map<string, TreeNode> | null,
    nextNodes: Map<string, TreeNode>,
    duration: number,
    now: number = performance.now()
  ): void {
    // Clear completed transitions
    this.cleanup(now);

    if (!prevNodes) {
      // First render: all nodes enter
      for (const node of nextNodes.values()) {
        this.transitions.set(node.id, {
          nodeId: node.id,
          type: "enter",
          startTime: now,
          duration,
          fromX: node.x,
          fromY: node.y,
          fromOpacity: 0,
          toX: node.x,
          toY: node.y,
          toOpacity: 1,
        });
      }
      return;
    }

    // Enter: nodes in next but not in prev
    for (const [id, node] of nextNodes) {
      if (!prevNodes.has(id)) {
        // New node: fade in from parent position or own position
        const parent = node.parentId ? nextNodes.get(node.parentId) : null;
        this.transitions.set(id, {
          nodeId: id,
          type: "enter",
          startTime: now,
          duration,
          fromX: parent ? parent.x + parent.width / 2 - node.width / 2 : node.x,
          fromY: parent ? parent.y + parent.height : node.y,
          fromOpacity: 0,
          toX: node.x,
          toY: node.y,
          toOpacity: 1,
        });
      }
    }

    // Update: nodes in both that moved
    for (const [id, node] of nextNodes) {
      const prev = prevNodes.get(id);
      if (prev && (prev.x !== node.x || prev.y !== node.y)) {
        this.transitions.set(id, {
          nodeId: id,
          type: "update",
          startTime: now,
          duration,
          fromX: prev.x,
          fromY: prev.y,
          fromOpacity: 1,
          toX: node.x,
          toY: node.y,
          toOpacity: 1,
        });
      }
    }

    // Exit: nodes in prev but not in next
    for (const [id, node] of prevNodes) {
      if (!nextNodes.has(id)) {
        this.transitions.set(id, {
          nodeId: id,
          type: "exit",
          startTime: now,
          duration,
          fromX: node.x,
          fromY: node.y,
          fromOpacity: 1,
          toX: node.x,
          toY: node.y,
          toOpacity: 0,
        });
      }
    }
  }

  /** Get interpolated position/opacity for a node. Returns null if no active transition. */
  getInterpolated(nodeId: string, now: number): InterpolatedState | null {
    const t = this.transitions.get(nodeId);
    if (!t) return null;

    const elapsed = now - t.startTime;
    const progress = Math.min(elapsed / t.duration, 1);
    const eased = easeOutCubic(progress);

    return {
      x: lerp(t.fromX, t.toX, eased),
      y: lerp(t.fromY, t.toY, eased),
      opacity: lerp(t.fromOpacity, t.toOpacity, eased),
    };
  }

  /** Are any transitions still active? */
  isAnimating(now: number = performance.now()): boolean {
    for (const t of this.transitions.values()) {
      if (now - t.startTime < t.duration) return true;
    }
    return false;
  }

  /** Remove completed transitions. */
  private cleanup(now: number): void {
    for (const [id, t] of this.transitions) {
      if (now - t.startTime >= t.duration) {
        this.transitions.delete(id);
      }
    }
  }

  /** Clear all transitions. */
  reset(): void {
    this.transitions.clear();
  }
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

function easeOutCubic(t: number): number {
  return 1 - Math.pow(1 - t, 3);
}

import type { TreeNode, TreeNodeKind, FunctionNodeData, VectorCompletionNodeData, LlmNodeData } from "../types";
import { scoreColor, SCORE_COLORS } from "../types";
import type { Viewport } from "./viewport";
import type { LodLevel, LodParams } from "./lod";
import { getLodParams } from "./lod";
import type { AnimationController, InterpolatedState } from "./animation";

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

export interface RenderTheme {
  bg: string;
  text: string;
  textSecondary: string;
  accent: string;
  nodeBg: string;
  nodeBorder: string;
  nodeSelectedBorder: string;
  edgeColor: string;
  edgeWidth: number;
  font: string;
  fontSmall: string;
  fontBold: string;
}

const LIGHT_THEME: RenderTheme = {
  bg: "#EDEDF2",
  text: "#1B1B1B",
  textSecondary: "#6B6B7B",
  accent: "#6B5CFF",
  nodeBg: "#FFFFFF",
  nodeBorder: "#D1D1D9",
  nodeSelectedBorder: "#6B5CFF",
  edgeColor: "#B0B0BE",
  edgeWidth: 1.5,
  font: '13px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  fontSmall: '11px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  fontBold: 'bold 13px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
};

const DARK_THEME: RenderTheme = {
  bg: "#1B1B1B",
  text: "#EDEDF2",
  textSecondary: "#9B9BAB",
  accent: "#6B5CFF",
  nodeBg: "#2A2A2E",
  nodeBorder: "#3A3A42",
  nodeSelectedBorder: "#6B5CFF",
  edgeColor: "#4A4A56",
  edgeWidth: 1.5,
  font: '13px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  fontSmall: '11px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  fontBold: 'bold 13px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
};

export function resolveTheme(mode: "light" | "dark" | "auto"): RenderTheme {
  if (mode === "light") return LIGHT_THEME;
  if (mode === "dark") return DARK_THEME;

  // Auto: check site's data-theme attribute first, then system preference
  if (typeof document !== "undefined") {
    const dataTheme = document.documentElement.getAttribute("data-theme");
    if (dataTheme === "dark") return DARK_THEME;
    if (dataTheme === "light") return LIGHT_THEME;
    if (document.documentElement.classList.contains("dark")) return DARK_THEME;
  }
  if (typeof window !== "undefined") {
    const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    return isDark ? DARK_THEME : LIGHT_THEME;
  }
  return LIGHT_THEME;
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

export class TreeRenderer {
  private textCache = new Map<string, number>();

  constructor(private ctx: CanvasRenderingContext2D) {}

  /** Clear the canvas and draw the full tree. */
  render(
    nodes: Map<string, TreeNode>,
    rootId: string,
    viewport: Viewport,
    theme: RenderTheme,
    lod: LodLevel,
    animation: AnimationController | null,
    selectedId: string | null,
    hoveredId: string | null,
    canvasWidth: number,
    canvasHeight: number
  ): void {
    const ctx = this.ctx;
    const params = getLodParams(lod);
    const now = performance.now();
    const dpr = (typeof window !== "undefined" ? window.devicePixelRatio : 1) || 1;

    // Clear the full physical canvas, then restore DPR scaling
    ctx.resetTransform();
    ctx.clearRect(0, 0, canvasWidth * dpr, canvasHeight * dpr);
    ctx.scale(dpr, dpr);
    ctx.fillStyle = theme.bg;
    ctx.fillRect(0, 0, canvasWidth, canvasHeight);

    // Apply viewport transform (multiplies with DPR scale)
    viewport.applyTransform(ctx);

    // Draw edges first (below nodes)
    if (params.showEdges) {
      this.drawEdges(nodes, viewport, theme, params, animation, now, canvasWidth, canvasHeight);
    }

    // Draw nodes
    if (lod === "dots") {
      this.drawDots(nodes, viewport, theme, params, animation, now, canvasWidth, canvasHeight);
    } else {
      this.drawNodes(nodes, viewport, theme, params, animation, now, selectedId, hoveredId, canvasWidth, canvasHeight);
    }
  }

  // -- Edges ----------------------------------------------------------------

  private drawEdges(
    nodes: Map<string, TreeNode>,
    viewport: Viewport,
    theme: RenderTheme,
    params: LodParams,
    animation: AnimationController | null,
    now: number,
    canvasWidth: number,
    canvasHeight: number
  ): void {
    const ctx = this.ctx;
    ctx.strokeStyle = theme.edgeColor;
    ctx.lineWidth = theme.edgeWidth;

    ctx.beginPath();

    for (const node of nodes.values()) {
      if (node.children.length === 0) continue;

      const parentState = animation?.getInterpolated(node.id, now);
      const px = (parentState?.x ?? node.x) + node.width / 2;
      const py = (parentState?.y ?? node.y) + node.height;

      for (const childId of node.children) {
        const child = nodes.get(childId);
        if (!child) continue;

        const childState = animation?.getInterpolated(childId, now);
        const cx = (childState?.x ?? child.x) + child.width / 2;
        const cy = childState?.y ?? child.y;

        // Viewport culling: skip if both endpoints are off-screen
        // (rough check using the parent and child positions)
        if (!this.edgeVisible(px, py, cx, cy, viewport, canvasWidth, canvasHeight)) {
          continue;
        }

        ctx.moveTo(px, py);
        if (params.curvedEdges) {
          const midY = py + (cy - py) / 2;
          ctx.bezierCurveTo(px, midY, cx, midY, cx, cy);
        } else {
          ctx.lineTo(cx, cy);
        }
      }
    }

    ctx.stroke();
  }

  private edgeVisible(
    x1: number, y1: number, x2: number, y2: number,
    viewport: Viewport,
    canvasWidth: number, canvasHeight: number
  ): boolean {
    const s1 = viewport.worldToScreen(x1, y1);
    const s2 = viewport.worldToScreen(x2, y2);
    const margin = 50;

    // If both endpoints are beyond the same edge, skip
    if (s1.x < -margin && s2.x < -margin) return false;
    if (s1.x > canvasWidth + margin && s2.x > canvasWidth + margin) return false;
    if (s1.y < -margin && s2.y < -margin) return false;
    if (s1.y > canvasHeight + margin && s2.y > canvasHeight + margin) return false;

    return true;
  }

  // -- Dots (LOD: dots) -----------------------------------------------------

  private drawDots(
    nodes: Map<string, TreeNode>,
    viewport: Viewport,
    theme: RenderTheme,
    params: LodParams,
    animation: AnimationController | null,
    now: number,
    canvasWidth: number,
    canvasHeight: number
  ): void {
    const ctx = this.ctx;
    const size = params.dotSize / viewport.zoom; // Constant screen-space size

    for (const node of nodes.values()) {
      const state = animation?.getInterpolated(node.id, now);
      const x = state?.x ?? node.x;
      const y = state?.y ?? node.y;
      const opacity = state?.opacity ?? 1;

      if (!viewport.isVisible(x, y, node.width, node.height, canvasWidth, canvasHeight)) {
        continue;
      }

      ctx.globalAlpha = opacity;
      ctx.fillStyle = this.nodeColor(node, theme);
      ctx.fillRect(
        x + node.width / 2 - size / 2,
        y + node.height / 2 - size / 2,
        size,
        size
      );
    }

    ctx.globalAlpha = 1;
  }

  // -- Nodes (LOD: full/simplified) -----------------------------------------

  private drawNodes(
    nodes: Map<string, TreeNode>,
    viewport: Viewport,
    theme: RenderTheme,
    params: LodParams,
    animation: AnimationController | null,
    now: number,
    selectedId: string | null,
    hoveredId: string | null,
    canvasWidth: number,
    canvasHeight: number
  ): void {
    const ctx = this.ctx;

    for (const node of nodes.values()) {
      const state = animation?.getInterpolated(node.id, now);
      const x = state?.x ?? node.x;
      const y = state?.y ?? node.y;
      const opacity = state?.opacity ?? 1;

      if (!viewport.isVisible(x, y, node.width, node.height, canvasWidth, canvasHeight)) {
        continue;
      }

      ctx.globalAlpha = opacity;

      const isSelected = node.id === selectedId;
      const isHovered = node.id === hoveredId;

      // Node background
      this.drawRoundedRect(
        x, y, node.width, node.height,
        params.cornerRadius,
        theme.nodeBg,
        isSelected
          ? theme.nodeSelectedBorder
          : isHovered
            ? theme.accent
            : theme.nodeBorder,
        isSelected || isHovered ? 2 : 1
      );

      // Kind-specific rendering
      switch (node.data.kind) {
        case "function":
          this.drawFunctionNode(node, x, y, theme, params);
          break;
        case "vector-completion":
          this.drawVectorCompletionNode(node, x, y, theme, params);
          break;
        case "llm":
          this.drawLlmNode(node, x, y, theme, params);
          break;
      }

      // State indicator (top-right corner)
      this.drawStateIndicator(node.state, x + node.width - 12, y + 8, theme);
    }

    ctx.globalAlpha = 1;
  }

  // -- Node type renderers --------------------------------------------------

  private drawFunctionNode(
    node: TreeNode,
    x: number, y: number,
    theme: RenderTheme,
    params: LodParams
  ): void {
    const ctx = this.ctx;
    const data = node.data as FunctionNodeData;
    const padding = 10;

    // Accent left stripe
    ctx.fillStyle = theme.accent;
    ctx.fillRect(x, y, 4, node.height);

    // Label
    if (params.showLabels) {
      ctx.font = theme.fontBold;
      ctx.fillStyle = theme.text;
      const label = params.maxLabelLength > 0
        ? truncate(node.label, params.maxLabelLength)
        : node.label;
      ctx.fillText(label, x + padding + 4, y + 22, node.width - padding * 2);
    }

    // Output score
    if (data.output !== null && params.showScoreBars) {
      const outputStr = typeof data.output === "number"
        ? `${(data.output * 100).toFixed(1)}%`
        : `[${(data.output as number[]).map(v => v.toFixed(2)).join(", ")}]`;

      ctx.font = theme.font;
      ctx.fillStyle = typeof data.output === "number"
        ? scoreColor(data.output)
        : theme.textSecondary;
      ctx.fillText(outputStr, x + padding + 4, y + 42, node.width - padding * 2);
    }

    // Task count
    if (params.showLabels) {
      ctx.font = theme.fontSmall;
      ctx.fillStyle = theme.textSecondary;
      ctx.fillText(`${data.taskCount} tasks`, x + padding + 4, y + 60, node.width - padding * 2);
    }
  }

  private drawVectorCompletionNode(
    node: TreeNode,
    x: number, y: number,
    theme: RenderTheme,
    params: LodParams
  ): void {
    const ctx = this.ctx;
    const data = node.data as VectorCompletionNodeData;
    const padding = 10;

    // Label
    if (params.showLabels) {
      ctx.font = theme.fontBold;
      ctx.fillStyle = theme.text;
      const label = params.maxLabelLength > 0
        ? truncate(node.label, params.maxLabelLength)
        : node.label;
      ctx.fillText(label, x + padding, y + 20, node.width - padding * 2);
    }

    // Score bar
    if (data.scores && data.scores.length > 0 && params.showScoreBars) {
      const maxScore = Math.max(...data.scores);
      const barY = y + 32;
      const barWidth = node.width - padding * 2;
      const barHeight = 6;

      // Background
      ctx.fillStyle = theme.nodeBorder;
      this.drawRoundedRectFill(x + padding, barY, barWidth, barHeight, 3);

      // Fill
      ctx.fillStyle = scoreColor(maxScore);
      this.drawRoundedRectFill(x + padding, barY, barWidth * maxScore, barHeight, 3);
    }

    // Vote count / status
    if (params.showLabels) {
      ctx.font = theme.fontSmall;
      if (data.voteCount > 0) {
        ctx.fillStyle = theme.textSecondary;
        ctx.fillText(`${data.voteCount} LLMs`, x + padding, y + 56, node.width - padding * 2);
      } else if (node.state === "pending") {
        ctx.fillStyle = theme.nodeBorder;
        ctx.fillText("Pending", x + padding, y + 56, node.width - padding * 2);
      } else if (node.state === "streaming") {
        ctx.fillStyle = theme.accent;
        ctx.fillText("Running\u2026", x + padding, y + 56, node.width - padding * 2);
      } else if (node.state === "error") {
        ctx.fillStyle = SCORE_COLORS.red;
        ctx.fillText("Error", x + padding, y + 56, node.width - padding * 2);
      } else {
        ctx.fillStyle = theme.textSecondary;
        ctx.fillText("No votes", x + padding, y + 56, node.width - padding * 2);
      }
    }
  }

  private drawLlmNode(
    node: TreeNode,
    x: number, y: number,
    theme: RenderTheme,
    params: LodParams
  ): void {
    const ctx = this.ctx;
    const data = node.data as LlmNodeData;
    const padding = 8;

    // Label (model name)
    if (params.showLabels) {
      ctx.font = theme.font;
      ctx.fillStyle = theme.text;
      const label = params.maxLabelLength > 0
        ? truncate(node.label, params.maxLabelLength)
        : node.label;
      ctx.fillText(label, x + padding, y + 16, node.width - padding * 2);
    }

    // Weight indicator
    if (params.showScoreBars) {
      const barY = y + 24;
      const barWidth = node.width - padding * 2;
      const barHeight = 4;

      ctx.fillStyle = theme.nodeBorder;
      this.drawRoundedRectFill(x + padding, barY, barWidth, barHeight, 2);

      ctx.fillStyle = theme.accent;
      this.drawRoundedRectFill(x + padding, barY, barWidth * Math.min(data.weight, 1), barHeight, 2);
    }

    // Streaming text preview
    if (params.showStreamingText && data.streamingText) {
      ctx.font = theme.fontSmall;
      ctx.fillStyle = theme.textSecondary;
      const preview = truncate(data.streamingText.replace(/\n/g, " "), 30);
      ctx.fillText(preview, x + padding, y + 44, node.width - padding * 2);
    }

    // Source badge (cache/rng)
    if (params.showLabels && (data.fromCache || data.fromRng)) {
      ctx.font = theme.fontSmall;
      ctx.fillStyle = data.fromRng ? SCORE_COLORS.orange : SCORE_COLORS.yellow;
      const badge = data.fromRng ? "RNG" : "CACHE";
      ctx.fillText(badge, x + node.width - padding - this.measureText(badge, theme.fontSmall), y + 16);
    }
  }

  // -- Helpers --------------------------------------------------------------

  private drawStateIndicator(
    state: string,
    x: number,
    y: number,
    theme: RenderTheme
  ): void {
    const ctx = this.ctx;
    const radius = 4;

    let color: string;
    switch (state) {
      case "complete": color = SCORE_COLORS.green; break;
      case "streaming": color = theme.accent; break;
      case "error": color = SCORE_COLORS.red; break;
      default: color = theme.nodeBorder; break;
    }

    ctx.beginPath();
    ctx.arc(x, y, radius, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();
  }

  private drawRoundedRect(
    x: number, y: number, w: number, h: number,
    r: number,
    fill: string,
    stroke: string,
    lineWidth: number
  ): void {
    const ctx = this.ctx;
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, r);
    ctx.fillStyle = fill;
    ctx.fill();
    ctx.strokeStyle = stroke;
    ctx.lineWidth = lineWidth;
    ctx.stroke();
  }

  private drawRoundedRectFill(
    x: number, y: number, w: number, h: number, r: number
  ): void {
    if (w <= 0) return;
    const ctx = this.ctx;
    ctx.beginPath();
    ctx.roundRect(x, y, Math.max(w, r * 2), h, r);
    ctx.fill();
  }

  private nodeColor(node: TreeNode, theme: RenderTheme): string {
    switch (node.kind) {
      case "function": return theme.accent;
      case "vector-completion": return SCORE_COLORS.green;
      case "llm": return SCORE_COLORS.yellow;
    }
  }

  private measureText(text: string, font: string): number {
    const key = `${font}:${text}`;
    let w = this.textCache.get(key);
    if (w === undefined) {
      this.ctx.font = font;
      w = this.ctx.measureText(text).width;
      this.textCache.set(key, w);
      // Limit cache size
      if (this.textCache.size > 500) {
        const firstKey = this.textCache.keys().next().value;
        if (firstKey) this.textCache.delete(firstKey);
      }
    }
    return w;
  }

  /** Clear the text measurement cache. */
  clearTextCache(): void {
    this.textCache.clear();
  }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

function truncate(text: string, maxLen: number): string {
  if (maxLen <= 0 || text.length <= maxLen) return text;
  return text.slice(0, maxLen - 1) + "\u2026";
}

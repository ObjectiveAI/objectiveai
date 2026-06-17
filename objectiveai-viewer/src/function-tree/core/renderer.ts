import type { TreeNode, FunctionNodeData, VectorCompletionNodeData, EnsembleLlmNodeData } from "../types";
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

const MONO_FONT = '"JetBrains Mono", "Fira Code", "SF Mono", "Cascadia Code", "Consolas", monospace';

const LIGHT_THEME: RenderTheme = {
  bg: "#faf8f5",
  text: "#1c1917",
  textSecondary: "#78716c",  // --copper-dim
  accent: "#d97706",          // --copper-mid
  nodeBg: "#f5f0eb",
  nodeBorder: "#d6d3d1",
  nodeSelectedBorder: "#d97706",
  edgeColor: "#a8a29e",       // --info-mid
  edgeWidth: 1,
  font: `13px ${MONO_FONT}`,
  fontSmall: `11px ${MONO_FONT}`,
  fontBold: `bold 13px ${MONO_FONT}`,
};

const DARK_THEME: RenderTheme = {
  bg: "#1c1917",           // --ground-surface (warm stone)
  text: "#d6d3d1",          // --info-bright
  textSecondary: "#78716c", // --copper-dim (brighter than info-dim for canvas readability)
  accent: "#d97706",        // --copper-mid
  nodeBg: "#141210",        // --ground-raised
  nodeBorder: "#292524",    // --node-border
  nodeSelectedBorder: "#d97706", // --copper-mid
  edgeColor: "#78716c",     // --copper-dim
  edgeWidth: 1,
  font: `13px ${MONO_FONT}`,
  fontSmall: `11px ${MONO_FONT}`,
  fontBold: `bold 13px ${MONO_FONT}`,
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
    canvasHeight: number,
    transparentBg: boolean = false
  ): void {
    const ctx = this.ctx;
    const params = getLodParams(lod);
    const now = performance.now();
    const dpr = (typeof window !== "undefined" ? window.devicePixelRatio : 1) || 1;

    // Clear the full physical canvas, then restore DPR scaling
    ctx.resetTransform();
    ctx.clearRect(0, 0, canvasWidth * dpr, canvasHeight * dpr);
    ctx.scale(dpr, dpr);
    if (!transparentBg) {
      ctx.fillStyle = theme.bg;
      ctx.fillRect(0, 0, canvasWidth, canvasHeight);
    }

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
      this.drawNodes(nodes, rootId, viewport, theme, params, animation, now, selectedId, hoveredId, canvasWidth, canvasHeight);
    }
  }

  // -- Edges ----------------------------------------------------------------

  private drawEdges(
    nodes: Map<string, TreeNode>,
    viewport: Viewport,
    theme: RenderTheme,
    _params: LodParams,
    animation: AnimationController | null,
    now: number,
    canvasWidth: number,
    canvasHeight: number
  ): void {
    const ctx = this.ctx;
    ctx.strokeStyle = theme.edgeColor;
    ctx.lineCap = "square";
    ctx.lineJoin = "miter";

    // Collect edges, grouping by line width for efficient batching.
    // Edges without weight data use the default width; weighted edges
    // are drawn individually since each may have a unique thickness.
    const defaultEdges: Array<[number, number, number, number]> = [];
    const weightedEdges: Array<[number, number, number, number, number]> = [];

    for (const node of nodes.values()) {
      if (node.children.length === 0) continue;

      const parentState = animation?.getInterpolated(node.id, now);
      const px = Math.round((parentState?.x ?? node.x) + node.width / 2) + 0.5;
      const py = Math.round((parentState?.y ?? node.y) + node.height) + 0.5;

      for (const childId of node.children) {
        const child = nodes.get(childId);
        if (!child) continue;

        const childState = animation?.getInterpolated(childId, now);
        const cx = Math.round((childState?.x ?? child.x) + child.width / 2) + 0.5;
        const cy = Math.round(childState?.y ?? child.y) + 0.5;

        // Viewport culling: skip if both endpoints are off-screen
        if (!this.edgeVisible(px, py, cx, cy, viewport, canvasWidth, canvasHeight)) {
          continue;
        }

        if (child.edgeWeight !== null) {
          // Map normalized weight [0, 1] to line width [1, 2]
          const w = 1 + child.edgeWeight;
          weightedEdges.push([px, py, cx, cy, w]);
        } else {
          defaultEdges.push([px, py, cx, cy]);
        }
      }
    }

    // Batch-draw all default-width edges in one path
    if (defaultEdges.length > 0) {
      ctx.lineWidth = theme.edgeWidth;
      ctx.beginPath();
      for (const [px, py, cx, cy] of defaultEdges) {
        const midY = py + (cy - py) / 2;
        ctx.moveTo(px, py);
        ctx.lineTo(px, midY);
        ctx.lineTo(cx, midY);
        ctx.lineTo(cx, cy);
      }
      ctx.stroke();
    }

    // Draw weighted edges — group by quantized width to reduce draw calls
    if (weightedEdges.length > 0) {
      // Sort by width so we can batch consecutive edges with the same width
      weightedEdges.sort((a, b) => a[4] - b[4]);
      let currentWidth = -1;

      for (const [px, py, cx, cy, w] of weightedEdges) {
        // Quantize to 0.25px increments to enable batching
        const qw = Math.round(w * 4) / 4;
        if (qw !== currentWidth) {
          if (currentWidth !== -1) ctx.stroke();
          currentWidth = qw;
          ctx.lineWidth = qw;
          ctx.beginPath();
        }
        const midY = py + (cy - py) / 2;
        ctx.moveTo(px, py);
        ctx.lineTo(px, midY);
        ctx.lineTo(cx, midY);
        ctx.lineTo(cx, cy);
      }
      ctx.stroke();
    }
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
      const x = Math.round(state?.x ?? node.x);
      const y = Math.round(state?.y ?? node.y);
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
    rootId: string,
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
      // Round to integer pixels for crisp text and shape edges
      const x = Math.round(state?.x ?? node.x);
      const y = Math.round(state?.y ?? node.y);
      const opacity = state?.opacity ?? 1;

      if (!viewport.isVisible(x, y, node.width, node.height, canvasWidth, canvasHeight)) {
        continue;
      }

      ctx.globalAlpha = opacity;

      const isSelected = node.id === selectedId;
      const isHovered = node.id === hoveredId;

      // Node background (dashed border for structural/pending nodes with no execution data)
      const isStructural = node.state === "pending" &&
        ((node.data.kind === "vector-completion" && node.data.voteCount === 0 && node.data.scores === null) ||
         (node.data.kind === "function" && node.data.output === null && node.data.profileId === null));
      const borderColor = isSelected
        ? theme.nodeSelectedBorder
        : isHovered
          ? theme.accent
          : theme.nodeBorder;
      const borderWidth = isSelected || isHovered ? 2 : 1;

      if (isStructural && !isSelected && !isHovered) {
        this.drawRoundedRectDashed(
          x, y, node.width, node.height,
          params.cornerRadius, theme.nodeBg, borderColor, borderWidth, [4, 3]
        );
      } else {
        this.drawRoundedRect(
          x, y, node.width, node.height,
          params.cornerRadius, theme.nodeBg, borderColor, borderWidth
        );
      }

      // Kind-specific rendering
      const isRoot = node.id === rootId;
      switch (node.data.kind) {
        case "function":
          this.drawFunctionNode(node, x, y, theme, params, isRoot);
          break;
        case "vector-completion":
          this.drawVectorCompletionNode(node, x, y, theme, params);
          break;
        case "ensemble-llm":
          this.drawEnsembleLlmNode(node, x, y, theme, params);
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
    params: LodParams,
    isRoot: boolean = false
  ): void {
    const ctx = this.ctx;
    const data = node.data as FunctionNodeData;
    const padding = 10;

    // Label
    if (params.showLabels) {
      ctx.font = theme.fontBold;
      ctx.fillStyle = theme.text;
      const label = params.maxLabelLength > 0
        ? truncate(node.label, params.maxLabelLength)
        : node.label;
      ctx.fillText(label, x + padding, y + 22, node.width - padding * 2);
    }

    const maxW = node.width - padding * 2;
    let cursorY = y + 30; // after label

    // Root node with output: prominent score display
    if (isRoot && data.output !== null && params.showScoreBars) {
      if (typeof data.output === "number") {
        const pct = data.output * 100;
        const color = scoreColor(data.output);

        // Large score text
        ctx.font = `bold 22px ${MONO_FONT}`;
        ctx.fillStyle = color;
        ctx.fillText(`${pct.toFixed(1)}%`, x + padding, cursorY + 18, maxW);
        cursorY += 26;

        // Score bar
        const barHeight = 5;
        ctx.fillStyle = theme.nodeBorder;
        this.drawRoundedRectFill(x + padding, cursorY, maxW, barHeight, 2.5);
        ctx.fillStyle = color;
        this.drawRoundedRectFill(x + padding, cursorY, maxW * data.output, barHeight, 2.5);
        cursorY += barHeight + 6;

        // Task count
        if (params.showLabels) {
          ctx.font = theme.fontSmall;
          ctx.fillStyle = theme.textSecondary;
          const typeLabel = data.functionType ? `${data.functionType} · ` : "";
          ctx.fillText(`${typeLabel}${data.taskCount} tasks`, x + padding, cursorY + 8, maxW);
          cursorY += 14;
        }
      } else {
        // Vector output — show ALL scores as mini bars (not just top 3)
        const scores = data.output as number[];
        const maxIdx = scores.indexOf(Math.max(...scores));
        const topScore = scores[maxIdx];
        const color = scoreColor(topScore);

        ctx.font = `bold 18px ${MONO_FONT}`;
        ctx.fillStyle = color;
        ctx.fillText(`#${maxIdx + 1} · ${(topScore * 100).toFixed(1)}%`, x + padding, cursorY + 14, maxW);
        cursorY += 22;

        // Show all scores as mini bars (cap at 6 for space)
        const visibleCount = Math.min(scores.length, 6);
        const sorted = scores.map((s, i) => ({ s, i })).sort((a, b) => b.s - a.s).slice(0, visibleCount);
        for (let si = 0; si < sorted.length; si++) {
          const barHeight = 4;
          ctx.fillStyle = theme.nodeBorder;
          this.drawRoundedRectFill(x + padding, cursorY, maxW, barHeight, 2);
          ctx.fillStyle = scoreColor(sorted[si].s);
          this.drawRoundedRectFill(x + padding, cursorY, maxW * sorted[si].s, barHeight, 2);
          cursorY += barHeight + 4;
        }
      }

      // Reasoning indicator (root only)
      if (data.reasoning && params.showLabels) {
        ctx.font = `bold 9px ${MONO_FONT}`;
        ctx.fillStyle = theme.accent;
        ctx.fillText("R", x + padding, cursorY + 8);
        ctx.font = `9px ${MONO_FONT}`;
        ctx.fillStyle = theme.textSecondary;
        ctx.fillText(truncate(data.reasoning, 28), x + padding + 12, cursorY + 8, maxW - 12);
        cursorY += 12;
      }

      // Execution ID (root only)
      if (data.executionId && params.showDetailBars) {
        ctx.font = `9px ${MONO_FONT}`;
        ctx.fillStyle = theme.textSecondary;
        ctx.fillText(`id:${truncate(data.executionId, 18)}`, x + padding, cursorY + 8, maxW);
      }

    } else if (data.output !== null && params.showScoreBars) {
      // Non-root with output
      if (typeof data.output === "number") {
        ctx.font = theme.font;
        ctx.fillStyle = scoreColor(data.output);
        ctx.fillText(`${(data.output * 100).toFixed(1)}%`, x + padding, cursorY + 10, maxW);
        cursorY += 16;

        // Mini score bar
        const barHeight = 4;
        ctx.fillStyle = theme.nodeBorder;
        this.drawRoundedRectFill(x + padding, cursorY, maxW, barHeight, 2);
        ctx.fillStyle = scoreColor(data.output);
        this.drawRoundedRectFill(x + padding, cursorY, maxW * data.output, barHeight, 2);
        cursorY += barHeight + 6;
      } else {
        // Non-root vector: show all mini bars
        const scores = data.output as number[];
        const visibleCount = Math.min(scores.length, 4);
        const sorted = scores.map((s, i) => ({ s, i })).sort((a, b) => b.s - a.s).slice(0, visibleCount);
        for (let si = 0; si < sorted.length; si++) {
          const barHeight = 4;
          ctx.fillStyle = theme.nodeBorder;
          this.drawRoundedRectFill(x + padding, cursorY, maxW, barHeight, 2);
          ctx.fillStyle = scoreColor(sorted[si].s);
          this.drawRoundedRectFill(x + padding, cursorY, maxW * sorted[si].s, barHeight, 2);
          cursorY += barHeight + 3;
        }
        cursorY += 3;
      }

      // Task count + function type
      if (params.showLabels) {
        ctx.font = theme.fontSmall;
        ctx.fillStyle = theme.textSecondary;
        const typeLabel = data.functionType ? `${data.functionType} · ` : "";
        ctx.fillText(`${typeLabel}${data.taskCount} tasks`, x + padding, cursorY + 8, maxW);
      }
    } else if (data.ownerRepo && params.showLabels) {
      ctx.font = theme.fontSmall;
      ctx.fillStyle = theme.textSecondary;
      ctx.fillText(data.ownerRepo, x + padding, cursorY + 10, maxW);
      cursorY += 16;

      ctx.font = theme.fontSmall;
      ctx.fillStyle = theme.textSecondary;
      const typeLabel = data.functionType ? `${data.functionType} · ` : "";
      ctx.fillText(`${typeLabel}${data.taskCount} tasks`, x + padding, cursorY + 8, maxW);
    } else if (params.showLabels) {
      ctx.font = theme.fontSmall;
      ctx.fillStyle = theme.textSecondary;
      const typeLabel = data.functionType ? `${data.functionType} · ` : "";
      ctx.fillText(`${typeLabel}${data.taskCount} tasks`, x + padding, cursorY + 10, maxW);
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
    const maxW = node.width - padding * 2;
    const bottom = y + node.height; // overflow guard
    let cy = y + 8; // cursorY — tracks vertical position

    // Label
    if (params.showLabels) {
      ctx.font = theme.fontBold;
      ctx.fillStyle = theme.text;
      const label = params.maxLabelLength > 0
        ? truncate(node.label, params.maxLabelLength)
        : node.label;
      ctx.fillText(label, x + padding, cy + 12, maxW);
      cy += 18;
    }

    // Prompt preview
    if (data.promptPreview && params.showLabels) {
      ctx.font = `italic 10px ${MONO_FONT}`;
      ctx.fillStyle = theme.textSecondary;
      ctx.fillText(truncate(data.promptPreview, 60), x + padding, cy + 10, maxW);
      cy += 15;
    }

    // Score bars
    if (data.scores && data.scores.length > 0 && params.showDetailBars) {
      // Full LOD: per-response bars with labels
      const barH = 8;
      const barGap = 4;
      const barW = maxW * 0.55;
      const labelX = x + padding + barW + 6;
      const labelW = maxW * 0.45 - 6;
      const count = Math.min(data.scores.length, 4);

      ctx.font = `10px ${MONO_FONT}`;
      cy += 4;

      for (let i = 0; i < count; i++) {
        if (cy + barH > bottom - 16) break; // leave room for status
        const score = data.scores[i];

        ctx.fillStyle = theme.nodeBorder;
        this.drawRoundedRectFill(x + padding, cy, barW, barH, 3);
        ctx.fillStyle = scoreColor(score);
        this.drawRoundedRectFill(x + padding, cy, barW * score, barH, 3);

        const lbl = data.responses?.[i] ? truncate(data.responses[i], 10) : `#${i + 1}`;
        ctx.fillStyle = theme.textSecondary;
        ctx.fillText(`${lbl} ${(score * 100).toFixed(1)}%`, labelX, cy + barH - 1, labelW);
        cy += barH + barGap;
      }

      if (data.scores.length > 4 && cy + 10 < bottom - 16) {
        ctx.fillStyle = theme.textSecondary;
        ctx.fillText(`+${data.scores.length - 4} more`, x + padding, cy + 8, maxW);
        cy += 12;
      }
    } else if (data.scores && data.scores.length > 0 && params.showScoreBars) {
      // Simplified LOD: single max-score bar
      const maxScore = Math.max(...data.scores);
      cy += 4;
      ctx.fillStyle = theme.nodeBorder;
      this.drawRoundedRectFill(x + padding, cy, maxW, 6, 3);
      ctx.fillStyle = scoreColor(maxScore);
      this.drawRoundedRectFill(x + padding, cy, maxW * maxScore, 6, 3);
      cy += 10;
    }

    // Status line
    if (params.showLabels && cy + 12 <= bottom) {
      ctx.font = theme.fontSmall;
      cy += 6;

      if (data.voteCount > 0) {
        ctx.fillStyle = theme.textSecondary;
        ctx.fillText(`${data.voteCount} LLMs`, x + padding, cy + 4, maxW);
      } else if (data.responseCount != null && data.responseCount > 0) {
        ctx.fillStyle = theme.textSecondary;
        ctx.fillText(`${data.responseCount} responses`, x + padding, cy + 4, maxW);
      } else if (node.state === "streaming") {
        const text = this.extractStreamingText(data);
        ctx.fillStyle = theme.accent;
        ctx.fillText(text ? truncate(text, 30) + "\u258C" : "Running\u2026", x + padding, cy + 4, maxW);
      } else if (node.state === "error") {
        ctx.fillStyle = SCORE_COLORS.error;
        ctx.fillText("Error", x + padding, cy + 4, maxW);
      } else if (node.state === "pending") {
        ctx.fillStyle = theme.nodeBorder;
        ctx.fillText("Pending", x + padding, cy + 4, maxW);
      } else {
        ctx.fillStyle = theme.textSecondary;
        ctx.fillText("No votes", x + padding, cy + 4, maxW);
      }
    }
  }

  /** Extract latest streaming text from completions array. */
  private extractStreamingText(data: VectorCompletionNodeData): string | null {
    if (!data.completions || data.completions.length === 0) return null;
    const last = data.completions[data.completions.length - 1];
    return last?.choices?.[0]?.delta?.content
      || last?.choices?.[0]?.message?.content
      || null;
  }

  private drawEnsembleLlmNode(
    node: TreeNode,
    x: number, y: number,
    theme: RenderTheme,
    params: LodParams
  ): void {
    const ctx = this.ctx;
    const data = node.data as EnsembleLlmNodeData;
    const padding = 8;
    const maxW = node.width - padding * 2;

    // Model name
    if (params.showLabels) {
      ctx.font = theme.fontSmall;
      ctx.fillStyle = theme.text;
      const label = params.maxLabelLength > 0
        ? truncate(node.label, params.maxLabelLength)
        : node.label;
      ctx.fillText(label, x + padding, y + 15, maxW);
    }

    // Weight + source badge
    if (params.showLabels) {
      ctx.font = theme.fontSmall;
      let info = `w=${data.weight.toFixed(2)}`;
      if (data.fromRng) info += " RNG";
      else if (data.fromCache) info += " CACHE";

      ctx.fillStyle = theme.textSecondary;
      ctx.fillText(info, x + padding, y + 27, maxW);

      // Output mode + logprobs on a separate compact line
      if (data.outputMode || data.topLogprobs) {
        let meta = "";
        if (data.outputMode) meta += data.outputMode.replace(/_/g, " ");
        if (data.topLogprobs) meta += meta ? ` · top${data.topLogprobs}` : `top${data.topLogprobs}`;
        ctx.font = `9px ${MONO_FONT}`;
        ctx.fillStyle = theme.textSecondary;
        ctx.fillText(meta, x + padding, y + 37, maxW);
      }
    }

    // Vote distribution sparkline (full LOD only)
    if (data.voteDistribution && data.voteDistribution.length > 0 && params.showDetailBars) {
      const hasMeta = !!(data.outputMode || data.topLogprobs);
      const sparkY = hasMeta ? y + 40 : y + 34;
      const sparkH = 16;
      const bw = 5;
      const gap = 1;

      // Baseline
      ctx.fillStyle = theme.nodeBorder;
      const totalBarW = data.voteDistribution.length * (bw + gap) - gap;
      ctx.fillRect(x + padding, sparkY + sparkH - 1, Math.min(totalBarW, maxW), 1);

      for (let i = 0; i < data.voteDistribution.length; i++) {
        const val = data.voteDistribution[i];
        const h = Math.max(2, val * sparkH);
        ctx.fillStyle = scoreColor(val);
        ctx.fillRect(x + padding + i * (bw + gap), sparkY + sparkH - h, bw, h);
      }
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
      case "complete": color = SCORE_COLORS.high; break;
      case "streaming": color = theme.accent; break;
      case "error": color = SCORE_COLORS.error; break;
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

  private drawRoundedRectDashed(
    x: number, y: number, w: number, h: number,
    r: number,
    fill: string,
    stroke: string,
    lineWidth: number,
    dash: number[]
  ): void {
    const ctx = this.ctx;
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, r);
    ctx.fillStyle = fill;
    ctx.fill();
    ctx.strokeStyle = stroke;
    ctx.lineWidth = lineWidth;
    ctx.setLineDash(dash);
    ctx.stroke();
    ctx.setLineDash([]);
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
      case "function": return theme.accent;  // copper-mid
      case "vector-completion": return "#b45309"; // copper-warm
      case "ensemble-llm": return theme.textSecondary;
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

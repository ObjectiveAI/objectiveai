import type {
  TreeNode,
  TreeData,
  FunctionTreeConfig,
  InputFunctionExecution,
  InputFunctionDefinition,
  InputProfile,
  VectorCompletionNodeData,
  EnsembleLlmNodeData,
} from "../types";
import { DEFAULT_CONFIG } from "../types";
import { buildTree, applyProfileWeights } from "./tree-data";
import { buildStructuralTree } from "./structural-tree-data";
import { layoutTree, treeBounds } from "./layout";
import { Viewport } from "./viewport";
import { TreeRenderer, resolveTheme, type RenderTheme } from "./renderer";
import { getLodLevel } from "./lod";
import { AnimationController } from "./animation";
import { InteractionHandler } from "./interaction";

// ---------------------------------------------------------------------------
// Engine: orchestrates layout, rendering, animation, and interaction
// ---------------------------------------------------------------------------

export class FunctionTreeEngine {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private viewport: Viewport;
  private renderer: TreeRenderer;
  private animation: AnimationController;
  private interaction: InteractionHandler;
  private config: FunctionTreeConfig;
  private theme: RenderTheme;

  private treeData: TreeData | null = null;
  private prevNodes: Map<string, TreeNode> | null = null;
  private selectedId: string | null = null;
  private hoveredId: string | null = null;
  private rafId: number | null = null;
  private needsRender = false;
  private destroyed = false;
  private userHasInteracted = false;

  // Layout debouncing
  private layoutTimer: ReturnType<typeof setTimeout> | null = null;
  private layoutPending = false;

  // Callbacks
  onNodeClick: ((node: TreeNode) => void) | null = null;
  onNodeHover: ((node: TreeNode | null) => void) | null = null;
  onSelectedNodeChange: ((node: TreeNode | null) => void) | null = null;

  constructor(canvas: HTMLCanvasElement, config?: Partial<FunctionTreeConfig>) {
    this.canvas = canvas;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Failed to get 2D rendering context");
    this.ctx = ctx;

    this.config = { ...DEFAULT_CONFIG, ...config };
    this.theme = resolveTheme(this.config.theme);
    this.viewport = new Viewport(this.config.minZoom, this.config.maxZoom);
    this.renderer = new TreeRenderer(this.ctx);
    this.animation = new AnimationController();

    this.interaction = new InteractionHandler(canvas, this.viewport, {
      onNodeClick: (node) => {
        this.selectedId = this.selectedId === node.id ? null : node.id;
        this.onNodeClick?.(node);
        this.onSelectedNodeChange?.(
          this.selectedId ? this.treeData?.nodes.get(this.selectedId) ?? null : null
        );
        this.requestRender();
      },
      onNodeHover: (node) => {
        this.hoveredId = node?.id ?? null;
        this.onNodeHover?.(node);
        this.requestRender();
      },
      onViewportChange: () => {
        this.userHasInteracted = true;
        this.requestRender();
      },
    });

    // Set initial cursor
    canvas.style.cursor = "grab";
  }

  // -- Public API -----------------------------------------------------------

  /** Update the tree data. Call on each streaming chunk or complete result. */
  setData(
    data: InputFunctionExecution | null,
    modelNames?: Record<string, string>,
    responseLabels?: Record<string, string[]>,
    profile?: InputProfile | null
  ): void {
    if (this.destroyed) return;

    const newTree = buildTree(data, modelNames, responseLabels);

    // Apply profile weights to edges (normalizes LLM weights + adds task weights)
    if (newTree) {
      applyProfileWeights(newTree, profile);

      // Enrich LLM nodes with profile metadata (outputMode, topLogprobs)
      if (profile?.tasks) {
        this.enrichLlmNodesFromProfile(newTree, profile);
      }
    }

    if (!newTree) {
      // Don't clear structural tree when execution data is null
      if (this.treeData?.mode !== "structural") {
        this.treeData = null;
        this.prevNodes = null;
        this.requestRender();
      }
      return;
    }

    // Preserve prompt data from structural tree (execution responses don't include prompts)
    if (this.prevNodes) {
      for (const [id, node] of newTree.nodes) {
        if (node.data.kind !== "vector-completion") continue;
        const prev = this.prevNodes.get(id);
        if (!prev || prev.data.kind !== "vector-completion") continue;
        const prevData = prev.data as VectorCompletionNodeData;
        if (prevData.promptPreview) {
          const execData = node.data as VectorCompletionNodeData;
          execData.promptPreview = prevData.promptPreview;
          execData.promptMessages = prevData.promptMessages;
          // Increase height to accommodate prompt (add 15px for prompt line)
          node.height += 15;
        }
      }
    }

    // Debounce layout during rapid streaming (max 3/sec)
    this.treeData = newTree;
    this.scheduleLayout();
  }

  /** Set a function definition for structural mode (shows task hierarchy before execution). */
  setDefinition(
    definition: InputFunctionDefinition | null,
    label?: string,
    resolvedSubFunctions?: Map<string, InputFunctionDefinition>,
  ): void {
    if (this.destroyed) return;

    const newTree = buildStructuralTree(definition, label, resolvedSubFunctions);

    if (!newTree) {
      // Only clear if we don't already have execution data
      if (this.treeData?.mode !== "execution") {
        this.treeData = null;
        this.prevNodes = null;
        this.requestRender();
      }
      return;
    }

    // Don't overwrite execution data with structural data
    if (this.treeData?.mode === "execution") return;

    this.treeData = newTree;
    this.scheduleLayout();
  }

  /** Update config (e.g., theme change). */
  setConfig(config: Partial<FunctionTreeConfig>): void {
    this.config = { ...this.config, ...config };
    this.theme = resolveTheme(this.config.theme);
    if (this.treeData) {
      layoutTree(this.treeData, this.config);
    }
    this.requestRender();
  }

  /** Resize the canvas (call from ResizeObserver). */
  resize(width: number, height: number): void {
    const dpr = window.devicePixelRatio || 1;
    this.canvas.width = width * dpr;
    this.canvas.height = height * dpr;
    this.canvas.style.width = `${width}px`;
    this.canvas.style.height = `${height}px`;
    this.ctx.scale(dpr, dpr);

    // Re-fit content when user hasn't manually panned/zoomed
    if (!this.userHasInteracted && this.treeData && width > 0 && height > 0) {
      this.viewport.fitToContent(this.treeData.nodes, width, height, 40, 1.0);
    }

    this.requestRender();
  }

  /** Zoom in by a fixed step. */
  zoomIn(): void {
    this.viewport.zoomAt(
      0.3,
      this.canvas.clientWidth / 2,
      this.canvas.clientHeight / 2
    );
    this.requestRender();
  }

  /** Zoom out by a fixed step. */
  zoomOut(): void {
    this.viewport.zoomAt(
      -0.3,
      this.canvas.clientWidth / 2,
      this.canvas.clientHeight / 2
    );
    this.requestRender();
  }

  /** Fit the entire tree in view. */
  fitToContent(): void {
    if (!this.treeData) return;
    this.userHasInteracted = false;
    this.viewport.fitToContent(
      this.treeData.nodes,
      this.canvas.clientWidth,
      this.canvas.clientHeight
    );
    this.requestRender();
  }

  /** Zoom to focus on a specific node. */
  zoomToNode(nodeId: string): void {
    const node = this.treeData?.nodes.get(nodeId);
    if (!node) return;

    this.viewport.animateTo({
      panX: node.x + node.width / 2 - this.canvas.clientWidth / 2,
      panY: node.y - 20,
      zoom: 1.2,
    });
    this.startRenderLoop();
  }

  /** Get the currently selected node. */
  getSelectedNode(): TreeNode | null {
    if (!this.selectedId || !this.treeData) return null;
    return this.treeData.nodes.get(this.selectedId) ?? null;
  }

  /** Deselect the current node. */
  deselect(): void {
    this.selectedId = null;
    this.onSelectedNodeChange?.(null);
    this.requestRender();
  }

  /** Clean up all resources. */
  destroy(): void {
    this.destroyed = true;
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
    if (this.layoutTimer !== null) {
      clearTimeout(this.layoutTimer);
      this.layoutTimer = null;
    }
    this.interaction.destroy();
    this.animation.reset();
  }

  /** Enrich ensemble-llm nodes with metadata from profile (outputMode, topLogprobs). */
  private enrichLlmNodesFromProfile(tree: TreeData, profile: InputProfile): void {
    const root = tree.nodes.get(tree.rootId);
    if (!root) return;

    for (let ti = 0; ti < root.children.length && ti < (profile.tasks?.length ?? 0); ti++) {
      const taskNode = tree.nodes.get(root.children[ti]);
      if (!taskNode) continue;

      const profileTask = profile.tasks[ti];
      if (!profileTask?.ensemble?.llms) continue;

      // Find LLM children of this task node
      let llmIdx = 0;
      for (const childId of taskNode.children) {
        const child = tree.nodes.get(childId);
        if (!child || child.kind !== "ensemble-llm") continue;

        const llmDef = profileTask.ensemble.llms[llmIdx];
        if (llmDef) {
          const llmData = child.data as EnsembleLlmNodeData;
          if (llmDef.output_mode) llmData.outputMode = llmDef.output_mode;
          if (llmDef.top_logprobs) llmData.topLogprobs = llmDef.top_logprobs;
        }
        llmIdx++;
      }
    }
  }

  // -- Internal -------------------------------------------------------------

  private scheduleLayout(): void {
    if (!this.treeData) return;

    if (this.layoutTimer !== null) {
      this.layoutPending = true;
      return;
    }

    this.executeLayout();

    // Debounce: min 333ms between layouts
    this.layoutTimer = setTimeout(() => {
      this.layoutTimer = null;
      if (this.layoutPending) {
        this.layoutPending = false;
        this.executeLayout();
      }
    }, 333);
  }

  private executeLayout(): void {
    if (!this.treeData) return;

    layoutTree(this.treeData, this.config);

    // Auto-fit only on first layout (no previous nodes), not during streaming updates.
    // This prevents jarring zoom changes as execution data streams in.
    // Cap auto zoom at 1.0 so small trees appear at natural size, not blown up.
    if (!this.userHasInteracted && !this.prevNodes && this.canvas.clientWidth > 0) {
      this.viewport.fitToContent(
        this.treeData.nodes,
        this.canvas.clientWidth,
        this.canvas.clientHeight,
        40,
        1.0
      );
    }

    // Schedule animations for changes
    if (this.config.animate) {
      this.animation.scheduleTransitions(
        this.prevNodes,
        this.treeData.nodes,
        this.config.animationDuration
      );
    }

    // Update interaction handler's node reference
    this.interaction.setNodes(this.treeData.nodes);

    // Save for next diff
    this.prevNodes = new Map(this.treeData.nodes);

    this.startRenderLoop();
  }

  private requestRender(): void {
    this.needsRender = true;
    if (this.rafId === null) {
      this.startRenderLoop();
    }
  }

  private startRenderLoop(): void {
    if (this.rafId !== null || this.destroyed) return;

    const loop = () => {
      if (this.destroyed) return;

      const now = performance.now();

      // Tick viewport animation
      const viewportAnimating = this.viewport.tick(now);

      // Check node animation
      const nodesAnimating = this.animation.isAnimating(now);

      // Render
      this.renderFrame();

      // Continue loop if animating or render requested
      if (viewportAnimating || nodesAnimating || this.needsRender) {
        this.needsRender = false;
        this.rafId = requestAnimationFrame(loop);
      } else {
        this.rafId = null;
      }
    };

    this.rafId = requestAnimationFrame(loop);
  }

  private renderFrame(): void {
    // Re-resolve theme on each frame for auto mode (picks up site theme changes)
    if (this.config.theme === "auto") {
      this.theme = resolveTheme("auto");
    }

    if (!this.treeData) {
      // Clear and draw empty state
      this.ctx.resetTransform();
      this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
      if (!this.config.transparentBg) {
        this.ctx.fillStyle = this.theme.bg;
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
      }
      return;
    }

    const lod = getLodLevel(this.viewport.zoom);

    this.renderer.render(
      this.treeData.nodes,
      this.treeData.rootId,
      this.viewport,
      this.theme,
      lod,
      this.config.animate ? this.animation : null,
      this.selectedId,
      this.hoveredId,
      this.canvas.clientWidth,
      this.canvas.clientHeight,
      this.config.transparentBg
    );
  }
}

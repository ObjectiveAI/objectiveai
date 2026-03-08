import { useRef, useEffect, useCallback } from "react";
import { FunctionTreeEngine } from "../core/engine";
import type { FunctionTreeConfig, InputFunctionExecution, TreeNode } from "../types";

interface UseEngineOptions {
  data: InputFunctionExecution | null;
  modelNames?: Record<string, string>;
  config?: Partial<FunctionTreeConfig>;
  onNodeClick?: (node: TreeNode) => void;
  onNodeHover?: (node: TreeNode | null) => void;
}

interface UseEngineResult {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  containerRef: React.RefObject<HTMLDivElement | null>;
  engine: FunctionTreeEngine | null;
  selectedNode: TreeNode | null;
  zoomIn: () => void;
  zoomOut: () => void;
  fitToContent: () => void;
  deselect: () => void;
}

export function useEngine(options: UseEngineOptions): UseEngineResult {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const engineRef = useRef<FunctionTreeEngine | null>(null);
  const selectedNodeRef = useRef<TreeNode | null>(null);

  // Create/destroy engine on mount/unmount
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const engine = new FunctionTreeEngine(canvas, options.config);
    engineRef.current = engine;

    // Initial resize
    const container = containerRef.current;
    if (container) {
      const rect = container.getBoundingClientRect();
      engine.resize(rect.width, rect.height);
    }

    return () => {
      engine.destroy();
      engineRef.current = null;
    };
    // Only re-create engine if config changes meaningfully
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ResizeObserver
  useEffect(() => {
    const container = containerRef.current;
    const engine = engineRef.current;
    if (!container || !engine) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (width > 0 && height > 0) {
          engine.resize(width, height);
        }
      }
    });

    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  // Sync data to engine
  useEffect(() => {
    const engine = engineRef.current;
    if (!engine) return;
    engine.setData(options.data, options.modelNames);
  }, [options.data, options.modelNames]);

  // Sync config
  useEffect(() => {
    const engine = engineRef.current;
    if (!engine || !options.config) return;
    engine.setConfig(options.config);
  }, [options.config]);

  // Sync callbacks
  useEffect(() => {
    const engine = engineRef.current;
    if (!engine) return;
    engine.onNodeClick = options.onNodeClick ?? null;
    engine.onNodeHover = options.onNodeHover ?? null;
    engine.onSelectedNodeChange = (node) => {
      selectedNodeRef.current = node;
    };
  }, [options.onNodeClick, options.onNodeHover]);

  const zoomIn = useCallback(() => engineRef.current?.zoomIn(), []);
  const zoomOut = useCallback(() => engineRef.current?.zoomOut(), []);
  const fitToContent = useCallback(() => engineRef.current?.fitToContent(), []);
  const deselect = useCallback(() => engineRef.current?.deselect(), []);

  return {
    canvasRef,
    containerRef,
    engine: engineRef.current,
    selectedNode: selectedNodeRef.current,
    zoomIn,
    zoomOut,
    fitToContent,
    deselect,
  };
}

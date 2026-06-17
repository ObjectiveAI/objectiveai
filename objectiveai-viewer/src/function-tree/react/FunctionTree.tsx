import React, { useState, useCallback, useEffect } from "react";
import type { FunctionTreeProps, TreeNode } from "../types";
import { useEngine } from "./use-engine";
import { Controls } from "./Controls";
import { DetailPanel } from "./DetailPanel";

/**
 * FunctionTree — 2D canvas visualization of ObjectiveAI function execution trees.
 *
 * Supports streaming data, pan/zoom, node selection, and Swiss system display.
 */
export function FunctionTree({
  data,
  definition,
  resolvedSubFunctions,
  profile,
  modelNames,
  responseLabels,
  config,
  onNodeClick,
  onNodeHover,
  width = "100%",
  height = 400,
  className,
  borderless = false,
}: FunctionTreeProps): React.ReactElement {
  const [selectedNode, setSelectedNode] = useState<TreeNode | null>(null);

  const handleNodeClick = useCallback(
    (node: TreeNode) => {
      setSelectedNode((prev) => (prev?.id === node.id ? null : node));
      onNodeClick?.(node);
    },
    [onNodeClick]
  );

  const {
    canvasRef,
    containerRef,
    zoomIn,
    zoomOut,
    fitToContent,
    deselect,
  } = useEngine({
    data,
    definition,
    resolvedSubFunctions,
    profile,
    modelNames,
    responseLabels,
    config,
    onNodeClick: handleNodeClick,
    onNodeHover,
  });

  // Dismiss detail panel on Escape
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && selectedNode) {
        setSelectedNode(null);
        deselect();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedNode, deselect]);

  const containerStyle: React.CSSProperties = {
    position: "relative",
    width: typeof width === "number" ? `${width}px` : width,
    height: typeof height === "number" ? `${height}px` : height,
    overflow: "hidden",
    ...(!borderless && {
      borderRadius: 4,
      border: "1px solid var(--ft-border, #D1D1D9)",
    }),
    ...(!borderless && {
      background: "var(--ft-bg, #EDEDF2)",
    }),
  };

  return (
    <div
      ref={containerRef}
      className={`ft-container${borderless ? " ft-borderless" : ""}${className ? ` ${className}` : ""}`}
      style={containerStyle}
    >
      <canvas
        ref={canvasRef}
        style={{
          display: "block",
          width: "100%",
          height: "100%",
        }}
      />

      <Controls
        onZoomIn={zoomIn}
        onZoomOut={zoomOut}
        onFitToContent={fitToContent}
      />

      {selectedNode && (
        <DetailPanel
          node={selectedNode}
          modelNames={modelNames}
          onClose={() => {
            setSelectedNode(null);
            deselect();
          }}
        />
      )}

      {!data && !definition && (
        <div className="ft-empty">
          <span className="ft-empty-text">
            Execute a function to see the tree
          </span>
        </div>
      )}
    </div>
  );
}

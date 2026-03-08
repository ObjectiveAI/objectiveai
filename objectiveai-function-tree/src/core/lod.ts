// ---------------------------------------------------------------------------
// Level of Detail: determines rendering fidelity based on zoom level
// ---------------------------------------------------------------------------

export type LodLevel = "full" | "simplified" | "dots";

/** Determine the LOD level based on the current zoom factor. */
export function getLodLevel(zoom: number): LodLevel {
  if (zoom >= 0.5) return "full";
  if (zoom >= 0.15) return "simplified";
  return "dots";
}

/** LOD-dependent rendering parameters. */
export interface LodParams {
  /** Whether to render edge curves (true) or straight lines (false). */
  curvedEdges: boolean;
  /** Whether to render node labels. */
  showLabels: boolean;
  /** Whether to render streaming text previews on LLM nodes. */
  showStreamingText: boolean;
  /** Whether to render score bars. */
  showScoreBars: boolean;
  /** Maximum label length (chars). 0 = full label. */
  maxLabelLength: number;
  /** Node corner radius. */
  cornerRadius: number;
  /** Whether to draw edges at all. */
  showEdges: boolean;
  /** Dot size when in "dots" mode. */
  dotSize: number;
}

export function getLodParams(level: LodLevel): LodParams {
  switch (level) {
    case "full":
      return {
        curvedEdges: true,
        showLabels: true,
        showStreamingText: true,
        showScoreBars: true,
        maxLabelLength: 0,
        cornerRadius: 8,
        showEdges: true,
        dotSize: 0,
      };
    case "simplified":
      return {
        curvedEdges: false,
        showLabels: true,
        showStreamingText: false,
        showScoreBars: false,
        maxLabelLength: 12,
        cornerRadius: 4,
        showEdges: true,
        dotSize: 0,
      };
    case "dots":
      return {
        curvedEdges: false,
        showLabels: false,
        showStreamingText: false,
        showScoreBars: false,
        maxLabelLength: 0,
        cornerRadius: 0,
        showEdges: false,
        dotSize: 6,
      };
  }
}

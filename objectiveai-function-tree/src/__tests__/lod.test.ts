import { describe, it, expect } from "vitest";
import { getLodLevel, getLodParams } from "../core/lod";

describe("getLodLevel", () => {
  it("returns 'full' for zoom >= 0.5", () => {
    expect(getLodLevel(0.5)).toBe("full");
    expect(getLodLevel(1.0)).toBe("full");
    expect(getLodLevel(3.0)).toBe("full");
  });

  it("returns 'simplified' for zoom in [0.15, 0.5)", () => {
    expect(getLodLevel(0.15)).toBe("simplified");
    expect(getLodLevel(0.3)).toBe("simplified");
    expect(getLodLevel(0.49)).toBe("simplified");
  });

  it("returns 'dots' for zoom < 0.15", () => {
    expect(getLodLevel(0.14)).toBe("dots");
    expect(getLodLevel(0.05)).toBe("dots");
    expect(getLodLevel(0.02)).toBe("dots");
  });

  it("boundary: exactly 0.5 is full", () => {
    expect(getLodLevel(0.5)).toBe("full");
  });

  it("boundary: exactly 0.15 is simplified", () => {
    expect(getLodLevel(0.15)).toBe("simplified");
  });
});

describe("getLodParams", () => {
  it("full: all visual features enabled", () => {
    const p = getLodParams("full");
    expect(p.curvedEdges).toBe(true);
    expect(p.showLabels).toBe(true);
    expect(p.showStreamingText).toBe(true);
    expect(p.showScoreBars).toBe(true);
    expect(p.showEdges).toBe(true);
    expect(p.maxLabelLength).toBe(0);
    expect(p.cornerRadius).toBe(8);
    expect(p.dotSize).toBe(0);
  });

  it("simplified: no streaming text or score bars, truncated labels", () => {
    const p = getLodParams("simplified");
    expect(p.curvedEdges).toBe(false);
    expect(p.showLabels).toBe(true);
    expect(p.showStreamingText).toBe(false);
    expect(p.showScoreBars).toBe(false);
    expect(p.maxLabelLength).toBe(12);
    expect(p.cornerRadius).toBe(4);
    expect(p.showEdges).toBe(true);
  });

  it("dots: minimal rendering, no labels or edges", () => {
    const p = getLodParams("dots");
    expect(p.showLabels).toBe(false);
    expect(p.showEdges).toBe(false);
    expect(p.dotSize).toBe(6);
    expect(p.curvedEdges).toBe(false);
    expect(p.showStreamingText).toBe(false);
    expect(p.showScoreBars).toBe(false);
  });
});

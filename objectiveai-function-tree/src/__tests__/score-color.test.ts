import { describe, it, expect } from "vitest";
import { scoreColor, SCORE_COLORS } from "../types";

describe("scoreColor", () => {
  it("returns green for scores >= 0.66", () => {
    expect(scoreColor(0.66)).toBe(SCORE_COLORS.green);
    expect(scoreColor(0.8)).toBe(SCORE_COLORS.green);
    expect(scoreColor(1.0)).toBe(SCORE_COLORS.green);
  });

  it("returns yellow for scores in [0.33, 0.66)", () => {
    expect(scoreColor(0.33)).toBe(SCORE_COLORS.yellow);
    expect(scoreColor(0.5)).toBe(SCORE_COLORS.yellow);
    expect(scoreColor(0.659)).toBe(SCORE_COLORS.yellow);
  });

  it("returns orange for scores in [0.15, 0.33)", () => {
    expect(scoreColor(0.15)).toBe(SCORE_COLORS.orange);
    expect(scoreColor(0.25)).toBe(SCORE_COLORS.orange);
    expect(scoreColor(0.329)).toBe(SCORE_COLORS.orange);
  });

  it("returns red for scores < 0.15", () => {
    expect(scoreColor(0.0)).toBe(SCORE_COLORS.red);
    expect(scoreColor(0.1)).toBe(SCORE_COLORS.red);
    expect(scoreColor(0.149)).toBe(SCORE_COLORS.red);
  });

  it("boundary: exactly 0.66 is green", () => {
    expect(scoreColor(0.66)).toBe(SCORE_COLORS.green);
  });

  it("boundary: exactly 0.33 is yellow", () => {
    expect(scoreColor(0.33)).toBe(SCORE_COLORS.yellow);
  });

  it("boundary: exactly 0.15 is orange", () => {
    expect(scoreColor(0.15)).toBe(SCORE_COLORS.orange);
  });
});

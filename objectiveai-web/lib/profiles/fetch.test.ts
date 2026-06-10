import { describe, it, expect } from "vitest";
import { classifyTier, buildTiers } from "./fetch";
import type { ProfileLlm } from "./types";

describe("classifyTier", () => {
  it("returns budget when maxWeight is 0", () => {
    expect(classifyTier(0, 0)).toBe("budget");
  });

  it("returns frontier for ratio >= 0.9", () => {
    expect(classifyTier(0.9, 1.0)).toBe("frontier");
    expect(classifyTier(1.0, 1.0)).toBe("frontier");
  });

  it("returns mid for ratio >= 0.2 and < 0.9", () => {
    expect(classifyTier(0.2, 1.0)).toBe("mid");
    expect(classifyTier(0.5, 1.0)).toBe("mid");
    expect(classifyTier(0.89, 1.0)).toBe("mid");
  });

  it("returns budget for ratio < 0.2", () => {
    expect(classifyTier(0.1, 1.0)).toBe("budget");
    expect(classifyTier(0.19, 1.0)).toBe("budget");
  });
});

describe("buildTiers", () => {
  const makeLlm = (model: string): ProfileLlm => ({
    model,
    outputMode: "default",
    topLogprobs: null,
    temperature: null,
    reasoning: null,
    count: 1,
    fallbacks: [],
  });

  it("sorts LLMs into correct tiers by weight", () => {
    const llms = [makeLlm("gpt-4o"), makeLlm("gpt-4o-mini"), makeLlm("deepseek-v3")];
    const weights = [1.0, 0.5, 0.1];

    const result = buildTiers(llms, weights);

    expect(result.frontier).toHaveLength(1);
    expect(result.frontier[0].llm.model).toBe("gpt-4o");
    expect(result.frontier[0].weight).toBe(1.0);

    expect(result.mid).toHaveLength(1);
    expect(result.mid[0].llm.model).toBe("gpt-4o-mini");

    expect(result.budget).toHaveLength(1);
    expect(result.budget[0].llm.model).toBe("deepseek-v3");
  });

  it("handles empty arrays", () => {
    const result = buildTiers([], []);
    expect(result.frontier).toHaveLength(0);
    expect(result.mid).toHaveLength(0);
    expect(result.budget).toHaveLength(0);
  });

  it("handles all equal weights as frontier", () => {
    const llms = [makeLlm("a"), makeLlm("b")];
    const weights = [1.0, 1.0];

    const result = buildTiers(llms, weights);
    expect(result.frontier).toHaveLength(2);
    expect(result.mid).toHaveLength(0);
    expect(result.budget).toHaveLength(0);
  });
});

import { describe, it, expect } from "vitest";
import {
  scoreColor,
  pct,
  dotPct,
  stateColor,
  taskState,
  funcState,
  normalizeType,
  shortType,
  labelFor,
  promptPreview,
  exprStr,
  getTaskAgents,
  getTaskWeight,
} from "./judgment-stack-utils";
import type { TaskExecution, FunctionExecution } from "./judgment-stack-utils";

describe("scoreColor", () => {
  it("returns copper-hot for >= 0.5", () => {
    expect(scoreColor(0.5)).toBe("var(--copper-hot)");
    expect(scoreColor(1.0)).toBe("var(--copper-hot)");
  });

  it("returns copper-mid for >= 0.3", () => {
    expect(scoreColor(0.3)).toBe("var(--copper-mid)");
    expect(scoreColor(0.49)).toBe("var(--copper-mid)");
  });

  it("returns copper-warm for >= 0.15", () => {
    expect(scoreColor(0.15)).toBe("var(--copper-warm)");
    expect(scoreColor(0.29)).toBe("var(--copper-warm)");
  });

  it("returns copper-dim for < 0.15", () => {
    expect(scoreColor(0.0)).toBe("var(--copper-dim)");
    expect(scoreColor(0.14)).toBe("var(--copper-dim)");
  });
});

describe("pct", () => {
  it("formats as percentage with one decimal", () => {
    expect(pct(0.5)).toBe("50.0%");
    expect(pct(1.0)).toBe("100.0%");
    expect(pct(0.0)).toBe("0.0%");
    expect(pct(0.123)).toBe("12.3%");
  });
});

describe("dotPct", () => {
  it("formats as dot-prefixed percentage", () => {
    expect(dotPct(0.5)).toBe(".50");
    expect(dotPct(1.0)).toBe(".100");
    expect(dotPct(0.0)).toBe(".00");
    expect(dotPct(0.05)).toBe(".05");
  });
});

describe("stateColor", () => {
  it("maps known states to colors", () => {
    expect(stateColor("complete")).toBe("var(--copper-hot)");
    expect(stateColor("streaming")).toBe("var(--copper-mid)");
    expect(stateColor("error")).toBe("var(--error)");
  });

  it("returns node-border for unknown states", () => {
    expect(stateColor("pending")).toBe("var(--node-border)");
    expect(stateColor("structural")).toBe("var(--node-border)");
    expect(stateColor("")).toBe("var(--node-border)");
  });
});

describe("taskState", () => {
  it("returns structural for null/undefined", () => {
    expect(taskState(null)).toBe("structural");
    expect(taskState(undefined)).toBe("structural");
  });

  it("returns error when error present", () => {
    expect(taskState({ error: { message: "fail" } })).toBe("error");
  });

  it("returns complete when scores present", () => {
    expect(taskState({ scores: [0.5, 0.5] })).toBe("complete");
  });

  it("returns streaming when completions present but no scores", () => {
    expect(taskState({ completions: [{}] })).toBe("streaming");
  });

  it("returns pending for empty execution", () => {
    expect(taskState({})).toBe("pending");
  });
});

describe("funcState", () => {
  it("returns structural for null/undefined", () => {
    expect(funcState(null)).toBe("structural");
    expect(funcState(undefined)).toBe("structural");
  });

  it("returns error when error present", () => {
    expect(funcState({ error: "something" })).toBe("error");
  });

  it("returns complete when output present", () => {
    expect(funcState({ output: 0.5 })).toBe("complete");
    expect(funcState({ output: [0.3, 0.7] })).toBe("complete");
  });

  it("returns streaming when tasks present but no output", () => {
    expect(funcState({ tasks: [{}] })).toBe("streaming");
  });

  it("returns pending for empty execution", () => {
    expect(funcState({})).toBe("pending");
  });
});

describe("normalizeType", () => {
  it("strips alpha prefix", () => {
    expect(normalizeType("alpha.vector.completion")).toBe("vector.completion");
    expect(normalizeType("alpha.scalar.function")).toBe("scalar.function");
  });

  it("preserves non-alpha types", () => {
    expect(normalizeType("vector.completion")).toBe("vector.completion");
  });
});

describe("shortType", () => {
  it("abbreviates vector.completion to vc", () => {
    expect(shortType("alpha.vector.completion")).toBe("vc");
    expect(shortType("vector.completion")).toBe("vc");
  });

  it("abbreviates function types", () => {
    expect(shortType("alpha.vector.function")).toBe("vector fn");
    expect(shortType("alpha.scalar.function")).toBe("scalar fn");
  });

  it("handles placeholder", () => {
    expect(shortType("alpha.placeholder")).toBe("placeholder");
  });
});

describe("labelFor", () => {
  it("returns string responses directly", () => {
    expect(labelFor("hello", 0)).toBe("hello");
  });

  it("truncates long strings beyond 24 chars", () => {
    const long = "this is a very long response text that should be truncated";
    const result = labelFor(long, 0);
    expect(result).toBe("this is a very long re\u2026");
    expect(result.length).toBe(23);
  });

  it("extracts text from objects", () => {
    expect(labelFor({ text: "option A" }, 0)).toBe("option A");
  });

  it("shows [dynamic] for expression objects", () => {
    expect(labelFor({ $jmespath: "input.x" }, 0)).toBe("[dynamic]");
    expect(labelFor({ $starlark: "x + 1" }, 0)).toBe("[dynamic]");
  });

  it("falls back to numbered label", () => {
    expect(labelFor(42, 0)).toBe("#1");
    expect(labelFor(null, 2)).toBe("#3");
  });
});

describe("promptPreview", () => {
  it("returns null when no messages", () => {
    expect(promptPreview({ type: "alpha.vector.completion" } as never)).toBeNull();
  });

  it("extracts system message content", () => {
    const task = {
      type: "alpha.vector.completion",
      messages: [{ role: "system", content: "You are a judge." }],
    };
    expect(promptPreview(task as never)).toBe("You are a judge.");
  });

  it("truncates long content at 80 chars", () => {
    const long = "A".repeat(100);
    const task = {
      type: "alpha.vector.completion",
      messages: [{ role: "user", content: long }],
    };
    expect(promptPreview(task as never)).toBe("A".repeat(77) + "\u2026");
  });

  it("prefers system > developer > user", () => {
    const task = {
      type: "alpha.vector.completion",
      messages: [
        { role: "user", content: "user msg" },
        { role: "system", content: "system msg" },
      ],
    };
    expect(promptPreview(task as never)).toBe("system msg");
  });
});

describe("exprStr", () => {
  it("returns null for undefined", () => {
    expect(exprStr(undefined)).toBeNull();
  });

  it("formats jmespath expressions", () => {
    expect(exprStr({ $jmespath: "input.count" })).toBe("jmes: input.count");
  });

  it("formats short starlark expressions", () => {
    expect(exprStr({ $starlark: "x + 1" })).toBe("star: x + 1");
  });

  it("truncates long starlark expressions", () => {
    const long = "A".repeat(50);
    const result = exprStr({ $starlark: long });
    expect(result).toBe("star: " + "A".repeat(37) + "\u2026");
  });
});

describe("getTaskAgents", () => {
  it("returns null when no profile", () => {
    expect(getTaskAgents(null, 0)).toBeNull();
    expect(getTaskAgents(undefined, 0)).toBeNull();
  });

  it("returns llms and weights for auto profile", () => {
    const profile = {
      kind: "auto" as const,
      llms: [{ model: "gpt-4o", outputMode: "default", topLogprobs: null, temperature: null, reasoning: null, count: 1, fallbacks: [] }],
      weights: [1.0],
      taskConfigs: [],
      taskWeights: [],
    };
    const result = getTaskAgents(profile as never, 0);
    expect(result?.llms).toEqual(profile.llms);
    expect(result?.weights).toEqual([1.0]);
  });
});

describe("getTaskWeight", () => {
  it("returns null when no profile", () => {
    expect(getTaskWeight(null, 0, 3)).toBeNull();
  });

  it("returns equal weight for auto profile", () => {
    const profile = { kind: "auto" as const, taskWeights: [] };
    expect(getTaskWeight(profile as never, 0, 4)).toBeCloseTo(0.25);
    expect(getTaskWeight(profile as never, 1, 2)).toBeCloseTo(0.5);
  });

  it("returns null when total is 0", () => {
    const profile = { kind: "auto" as const, taskWeights: [] };
    expect(getTaskWeight(profile as never, 0, 0)).toBeNull();
  });
});

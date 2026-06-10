import { describe, it, expect } from "vitest";
import { parseType } from "./fetch";

describe("parseType", () => {
  it("parses scalar leaf", () => {
    const result = parseType("alpha.scalar");
    expect(result.category).toBe("scalar");
    expect(result.depth).toBe("leaf");
    expect(result.clean).toBe("scalar");
  });

  it("parses vector leaf function", () => {
    const result = parseType("alpha.vector.function");
    expect(result.category).toBe("vector");
    expect(result.depth).toBe("leaf");
    expect(result.clean).toBe("vector");
  });

  it("parses scalar branch function", () => {
    const result = parseType("alpha.scalar.branch.function");
    expect(result.category).toBe("scalar");
    expect(result.depth).toBe("branch");
    expect(result.clean).toBe("scalar.branch");
  });

  it("strips alpha prefix and .function suffix", () => {
    const result = parseType("alpha.vector.completion.function");
    expect(result.clean).toBe("vector.completion");
  });

  it("handles non-alpha prefixed types", () => {
    const result = parseType("vector.completion");
    expect(result.category).toBe("vector");
    expect(result.depth).toBe("completion");
    expect(result.clean).toBe("vector.completion");
  });
});

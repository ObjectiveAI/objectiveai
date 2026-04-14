import { render, screen, cleanup } from "@testing-library/react";
import { describe, it, expect, afterEach } from "vitest";
import { StructuralVc } from "./StructuralVc";

afterEach(cleanup);

describe("StructuralVc", () => {
  it("renders response tags", () => {
    render(
      <StructuralVc
        responses={["option A", "option B"]}
        agents={null}
      />
    );
    expect(screen.getByText("option A")).toBeInTheDocument();
    expect(screen.getByText("option B")).toBeInTheDocument();
  });

  it("renders agent weight bars", () => {
    render(
      <StructuralVc
        responses={[]}
        agents={{
          llms: [
            { model: "gpt-4o", outputMode: "default", topLogprobs: null, temperature: null, reasoning: null, count: 1, fallbacks: [] },
            { model: "claude-3.5-sonnet", outputMode: "log_probs", topLogprobs: 5, temperature: null, reasoning: null, count: 2, fallbacks: [] },
          ],
          weights: [1.0, 0.5],
        }}
      />
    );
    expect(screen.getByText("gpt-4o")).toBeInTheDocument();
    expect(screen.getByText("claude-3.5-sonnet \u00d72")).toBeInTheDocument();
    expect(screen.getByText("1.00")).toBeInTheDocument();
    expect(screen.getByText("0.50")).toBeInTheDocument();
  });

  it("shows empty state when no agents and no responses", () => {
    render(<StructuralVc responses={[]} agents={null} />);
    expect(screen.getByText("no configuration")).toBeInTheDocument();
  });

  it("strips provider prefix from model names", () => {
    render(
      <StructuralVc
        responses={[]}
        agents={{
          llms: [{ model: "openai/gpt-4o-mini", outputMode: "default", topLogprobs: null, temperature: null, reasoning: null, count: 1, fallbacks: [] }],
          weights: [1.0],
        }}
      />
    );
    expect(screen.getByText("gpt-4o-mini")).toBeInTheDocument();
  });
});

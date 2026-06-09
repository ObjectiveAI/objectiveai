import { render, screen, cleanup } from "@testing-library/react";
import { describe, it, expect, afterEach } from "vitest";
import { VoteMatrix } from "./VoteMatrix";

afterEach(cleanup);

describe("VoteMatrix", () => {
  const baseVotes = [
    { model: "gpt-4o", vote: [0.8, 0.2], weight: 1.0 },
    { model: "claude-3.5-sonnet", vote: [0.3, 0.7], weight: 0.5 },
  ];

  it("renders agent names", () => {
    render(
      <VoteMatrix
        votes={baseVotes}
        scores={[0.63, 0.37]}
        responses={["A", "B"]}
        agents={null}
      />
    );
    expect(screen.getByText("gpt-4o")).toBeInTheDocument();
    expect(screen.getByText("claude-3.5-sonnet")).toBeInTheDocument();
  });

  it("renders response column headers", () => {
    render(
      <VoteMatrix
        votes={baseVotes}
        scores={[0.63, 0.37]}
        responses={["option A", "option B"]}
        agents={null}
      />
    );
    expect(screen.getByText("option A")).toBeInTheDocument();
    expect(screen.getByText("option B")).toBeInTheDocument();
  });

  it("renders vote values as dot percentages", () => {
    render(
      <VoteMatrix
        votes={[{ model: "test", vote: [1.0, 0.0], weight: 1.0 }]}
        scores={[1.0, 0.0]}
        responses={["A", "B"]}
        agents={null}
      />
    );
    expect(screen.getByText(".100")).toBeInTheDocument();
    expect(screen.getByText(".00")).toBeInTheDocument();
  });

  it("renders convergence row with scores", () => {
    render(
      <VoteMatrix
        votes={baseVotes}
        scores={[0.63, 0.37]}
        responses={["A", "B"]}
        agents={null}
      />
    );
    expect(screen.getByText("63.0%")).toBeInTheDocument();
    expect(screen.getByText("37.0%")).toBeInTheDocument();
  });

  it("renders weight values", () => {
    render(
      <VoteMatrix
        votes={baseVotes}
        scores={[0.63, 0.37]}
        responses={["A", "B"]}
        agents={null}
      />
    );
    expect(screen.getByText("1.00")).toBeInTheDocument();
    expect(screen.getByText("0.50")).toBeInTheDocument();
  });

  it("shows cache and RNG flags", () => {
    const votes = [
      { model: "test", vote: [1.0], weight: 1.0, from_cache: true, from_rng: true },
    ];
    render(
      <VoteMatrix votes={votes} scores={[1.0]} responses={["A"]} agents={null} />
    );
    expect(screen.getByText("C")).toBeInTheDocument();
    expect(screen.getByText("R")).toBeInTheDocument();
  });

  it("strips provider prefix from model names", () => {
    const votes = [
      { model: "openai/gpt-4o-mini", vote: [0.5, 0.5], weight: 1.0 },
    ];
    render(
      <VoteMatrix votes={votes} scores={[0.5, 0.5]} responses={["A", "B"]} agents={null} />
    );
    expect(screen.getByText("gpt-4o-mini")).toBeInTheDocument();
  });
});

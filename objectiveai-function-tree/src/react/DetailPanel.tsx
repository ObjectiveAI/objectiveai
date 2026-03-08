import React from "react";
import type { TreeNode, FunctionNodeData, VectorCompletionNodeData, LlmNodeData } from "../types";
import { scoreColor } from "../types";

interface DetailPanelProps {
  node: TreeNode;
  modelNames?: Record<string, string>;
  onClose: () => void;
}

export function DetailPanel({ node, modelNames, onClose }: DetailPanelProps): React.ReactElement {
  return (
    <div className="ft-detail-panel" role="dialog" aria-label="Node details">
      <div className="ft-detail-header">
        <span className="ft-detail-kind">{kindLabel(node.kind)}</span>
        <button
          className="ft-detail-close"
          onClick={onClose}
          aria-label="Close"
        >
          &times;
        </button>
      </div>

      <h3 className="ft-detail-title">{node.label}</h3>

      <div className="ft-detail-state">
        <span
          className="ft-detail-state-dot"
          style={{ background: stateColor(node.state) }}
        />
        {node.state}
      </div>

      {node.data.kind === "function" && (
        <FunctionDetails data={node.data} />
      )}
      {node.data.kind === "vector-completion" && (
        <VectorCompletionDetails data={node.data} modelNames={modelNames} />
      )}
      {node.data.kind === "llm" && (
        <LlmDetails data={node.data} modelNames={modelNames} />
      )}
    </div>
  );
}

function FunctionDetails({ data }: { data: FunctionNodeData }): React.ReactElement {
  return (
    <div className="ft-detail-body">
      {data.functionId && (
        <DetailRow label="Function" value={data.functionId} />
      )}
      {data.profileId && (
        <DetailRow label="Profile" value={data.profileId} />
      )}
      <DetailRow label="Tasks" value={String(data.taskCount)} />
      {data.output !== null && (
        <DetailRow
          label="Output"
          value={formatOutput(data.output)}
          valueColor={
            typeof data.output === "number"
              ? scoreColor(data.output)
              : undefined
          }
        />
      )}
      {data.error && (
        <DetailRow label="Error" value={data.error} valueColor="rgb(239, 68, 68)" />
      )}
    </div>
  );
}

function VectorCompletionDetails({
  data,
  modelNames,
}: {
  data: VectorCompletionNodeData;
  modelNames?: Record<string, string>;
}): React.ReactElement {
  return (
    <div className="ft-detail-body">
      <DetailRow label="Task Index" value={data.taskPath.join(" > ")} />
      <DetailRow label="LLMs" value={String(data.voteCount)} />
      {data.scores && data.scores.length > 0 && (
        <div className="ft-detail-scores">
          <span className="ft-detail-label">Scores</span>
          <div className="ft-detail-score-bars">
            {data.scores.map((score, i) => (
              <div key={i} className="ft-detail-score-bar">
                <div
                  className="ft-detail-score-fill"
                  style={{
                    width: `${score * 100}%`,
                    background: scoreColor(score),
                  }}
                />
                <span className="ft-detail-score-label">
                  {(score * 100).toFixed(1)}%
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
      {data.votes && data.votes.length > 0 && (
        <div className="ft-detail-votes">
          <span className="ft-detail-label">Vote Breakdown ({data.votes.length})</span>
          <div className="ft-detail-vote-list">
            {data.votes.map((vote, i) => {
              const name = modelNames?.[vote.model]
                ? (modelNames[vote.model].split("/").pop() ?? vote.model.slice(0, 8))
                : vote.model.slice(0, 8);
              const maxVote = vote.vote.length > 0 ? Math.max(...vote.vote) : 0;
              return (
                <div key={i} className="ft-detail-vote-item">
                  <div className="ft-detail-vote-header">
                    <span className="ft-detail-vote-name">{name}</span>
                    <span className="ft-detail-vote-weight">w={vote.weight.toFixed(2)}</span>
                  </div>
                  {(vote.from_cache || vote.from_rng) && (
                    <span className={`ft-detail-vote-badge${vote.from_rng ? " ft-detail-vote-badge-rng" : ""}`}>
                      {vote.from_rng ? "RNG" : "CACHE"}
                    </span>
                  )}
                  {vote.vote.length > 0 && (
                    <div className="ft-detail-score-bar" style={{ height: 14, marginTop: 2 }}>
                      <div
                        className="ft-detail-score-fill"
                        style={{
                          width: `${maxVote * 100}%`,
                          background: scoreColor(maxVote),
                        }}
                      />
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}
      {data.error && (
        <DetailRow label="Error" value={data.error} valueColor="rgb(239, 68, 68)" />
      )}
    </div>
  );
}

function LlmDetails({
  data,
  modelNames,
}: {
  data: LlmNodeData;
  modelNames?: Record<string, string>;
}): React.ReactElement {
  const resolvedName = modelNames?.[data.modelId] ?? data.modelName;
  const maxVote = data.vote ? Math.max(...data.vote) : 0;

  return (
    <div className="ft-detail-body">
      <DetailRow
        label="Model"
        value={resolvedName ?? data.modelId}
      />
      <DetailRow label="Weight" value={data.weight.toFixed(3)} />

      {data.fromCache && <DetailRow label="Source" value="Cached" />}
      {data.fromRng && <DetailRow label="Source" value="RNG (simulated)" />}

      {data.vote && data.vote.length > 0 && (
        <div className="ft-detail-scores">
          <span className="ft-detail-label">Vote Distribution</span>
          <div className="ft-detail-score-bars">
            {data.vote.map((v, i) => (
              <div key={i} className="ft-detail-score-bar">
                <div
                  className="ft-detail-score-fill"
                  style={{
                    width: `${v * 100}%`,
                    background: scoreColor(v),
                  }}
                />
                <span className="ft-detail-score-label">
                  {(v * 100).toFixed(1)}%
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {data.streamingText && (
        <div className="ft-detail-text">
          <span className="ft-detail-label">Reasoning</span>
          <pre className="ft-detail-pre">{data.streamingText}</pre>
        </div>
      )}
    </div>
  );
}

// -- Helpers ----------------------------------------------------------------

function DetailRow({
  label,
  value,
  valueColor,
}: {
  label: string;
  value: string;
  valueColor?: string;
}): React.ReactElement {
  return (
    <div className="ft-detail-row">
      <span className="ft-detail-label">{label}</span>
      <span className="ft-detail-value" style={valueColor ? { color: valueColor } : undefined}>
        {value}
      </span>
    </div>
  );
}

function kindLabel(kind: string): string {
  switch (kind) {
    case "function": return "Function";
    case "vector-completion": return "Vector Completion";
    case "llm": return "LLM";
    default: return kind;
  }
}

function stateColor(state: string): string {
  switch (state) {
    case "complete": return "rgb(34, 197, 94)";
    case "streaming": return "#6B5CFF";
    case "error": return "rgb(239, 68, 68)";
    default: return "#B0B0BE";
  }
}

function formatOutput(output: number | number[]): string {
  if (typeof output === "number") {
    return `${(output * 100).toFixed(1)}%`;
  }
  return `[${output.map((v) => v.toFixed(3)).join(", ")}]`;
}

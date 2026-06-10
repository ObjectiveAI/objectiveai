import type { ProfileLlm } from "@/lib/profiles/types";
import { labelFor } from "@/lib/judgment-stack-utils";
import styles from "./StructuralVc.module.css";

export function StructuralVc({
  responses,
  agents,
}: {
  responses: unknown[];
  agents: { llms: ProfileLlm[]; weights: number[] } | null;
}) {
  const maxW = agents ? Math.max(...agents.weights, 0) || 1 : 1;

  return (
    <div>
      {responses.length > 0 && (
        <div className={styles.responses}>
          {responses.map((r, i) => (
            <span key={i} className={styles.responseTag}>
              {labelFor(r, i)}
            </span>
          ))}
        </div>
      )}

      {agents && agents.llms.length > 0 && (
        <div className={styles.structuralAgents}>
          {agents.llms.map((llm, i) => {
            const w = agents.weights[i] ?? 0;
            const rel = maxW > 0 ? w / maxW : 0;
            const name = llm.model.includes("/") ? llm.model.split("/").pop() : llm.model;
            return (
              <div key={i} className={styles.structuralAgent}>
                <div
                  className={styles.structuralAgentBar}
                  style={{
                    width: `${Math.max(20, rel * 80)}%`,
                    background: `rgba(245, 158, 11, ${0.2 + rel * 0.5})`,
                  }}
                />
                <span className={styles.structuralAgentName}>
                  {name}
                  {llm.count > 1 ? ` \u00d7${llm.count}` : ""}
                </span>
                <span className={styles.structuralAgentWeight}>{w.toFixed(2)}</span>
              </div>
            );
          })}
        </div>
      )}

      {!agents && responses.length === 0 && (
        <div className={styles.empty}>no configuration</div>
      )}
    </div>
  );
}

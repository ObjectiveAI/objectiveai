"use client";

import Link from "next/link";
import type { ProfileMeta } from "@/lib/profiles/types";
import styles from "./ProfileCard.module.css";

/** Short model name: "openai/gpt-4o-mini" → "gpt-4o-mini" */
function shortModel(model: string): string {
  return model.includes("/") ? model.split("/").pop()! : model;
}

export function ProfileCard({ profile }: { profile: ProfileMeta }) {
  const { tiers } = profile;
  const hasFrontier = tiers.frontier.length > 0;
  const hasMid = tiers.mid.length > 0;
  const hasBudget = tiers.budget.length > 0;

  const maxW = Math.max(...profile.weights, 0);

  return (
    <Link href={`/profiles/${profile.name}`} className={styles.card}>
      <div className={styles.cardBody}>
        <div className={styles.cardHeader}>
          <span className={styles.cardName}>{profile.name}</span>
          <span className={styles.agentBadge}>{profile.totalAgents}</span>
        </div>

        {/* Weight bar — segments colored by tier */}
        {profile.weights.length > 0 && (
          <div className={styles.weightBar}>
            {profile.weights.map((w, i) => {
              const ratio = maxW > 0 ? w / maxW : 0;
              return (
                <div
                  key={i}
                  className={styles.weightSegment}
                  style={{
                    flex: Math.max(w, 0.01),
                    opacity: 0.2 + ratio * 0.8,
                  }}
                />
              );
            })}
          </div>
        )}

        {/* Tier groups */}
        <div className={styles.tierGroups}>
          {hasFrontier && (
            <div className={styles.tierGroup}>
              <span className={`${styles.tierLabel} ${styles.tierFrontier}`}>frontier</span>
              <div className={styles.tierAgents}>
                {tiers.frontier.map(({ llm }, i) => (
                  <span key={i} className={`${styles.agentChip} ${styles.chipFrontier}`}>
                    {shortModel(llm.model)}
                    {llm.count > 1 && <span className={styles.chipCount}>×{llm.count}</span>}
                  </span>
                ))}
              </div>
            </div>
          )}
          {hasMid && (
            <div className={styles.tierGroup}>
              <span className={`${styles.tierLabel} ${styles.tierMid}`}>mid</span>
              <div className={styles.tierAgents}>
                {tiers.mid.map(({ llm }, i) => (
                  <span key={i} className={`${styles.agentChip} ${styles.chipMid}`}>
                    {shortModel(llm.model)}
                    {llm.count > 1 && <span className={styles.chipCount}>×{llm.count}</span>}
                  </span>
                ))}
              </div>
            </div>
          )}
          {hasBudget && (
            <div className={styles.tierGroup}>
              <span className={`${styles.tierLabel} ${styles.tierBudget}`}>budget</span>
              <div className={styles.tierAgents}>
                {tiers.budget.map(({ llm }, i) => (
                  <span key={i} className={styles.agentChip}>
                    {shortModel(llm.model)}
                    {llm.count > 1 && <span className={styles.chipCount}>×{llm.count}</span>}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </Link>
  );
}

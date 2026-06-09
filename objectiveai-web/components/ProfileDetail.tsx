"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import type { ProfileMeta, ProfileLlm } from "@/lib/profiles/types";
import { fetchProfileBySlug } from "@/lib/profiles/fetch";
import { MOCK_PROFILES } from "@/lib/TEMPORARY_mock_explore";
import styles from "./ProfileDetail.module.css";

interface Props {
  name: string;
}

function formatWeight(w: number): string {
  return (w * 100).toFixed(1) + "%";
}

function TierSection({
  label,
  agents,
  labelClass,
}: {
  label: string;
  agents: { llm: ProfileLlm; weight: number }[];
  labelClass?: string;
}) {
  if (agents.length === 0) return null;
  return (
    <div className={styles.tierSection}>
      <span className={`${styles.tierLabel} ${labelClass ?? ""}`}>{label}</span>
      {agents.map(({ llm, weight }, i) => (
        <div key={i} className={styles.agentCard}>
          <span className={styles.agentModel}>{llm.model}</span>
          <div className={styles.agentMeta}>
            <span className={`${styles.tag} ${styles.tagWeight}`}>
              {formatWeight(weight)}
            </span>
            {llm.count > 1 && (
              <span className={`${styles.tag} ${styles.tagCount}`}>
                ×{llm.count}
              </span>
            )}
            <span className={styles.tag}>{llm.outputMode}</span>
            {llm.topLogprobs != null && (
              <span className={styles.tag}>logprobs:{llm.topLogprobs}</span>
            )}
            {llm.temperature != null && (
              <span className={styles.tag}>t:{llm.temperature}</span>
            )}
            {llm.reasoning === true && (
              <span className={styles.tag}>reasoning</span>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

export function ProfileDetail({ name }: Props) {
  const [data, setData] = useState<ProfileMeta | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    // TEMPORARY — try mock data first while API is down
    const mock = MOCK_PROFILES.find((p) => p.name === name);
    if (mock) {
      setData(mock);
      setLoading(false);
      return;
    }

    fetchProfileBySlug("ObjectiveAI", `profile-${name}`)
      .then((d) => {
        if (!cancelled) { setData(d); setLoading(false); }
      })
      .catch((err) => {
        if (!cancelled) { setError(err.message); setLoading(false); }
      });
    return () => { cancelled = true; };
  }, [name, retryCount]);

  if (loading) {
    return (
      <div className={styles.loading} role="status" aria-live="polite">
        <span className={styles.loadingDot} />
        loading profile
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className={styles.error} role="alert">
        <span>unable to load profile</span>
        <button
          onClick={() => setRetryCount((c) => c + 1)}
          className={styles.retryBtn}
        >
          try again
        </button>
      </div>
    );
  }

  const { tiers } = data;
  const weightMax = Math.max(...data.weights, 0);

  return (
    <div className={styles.detail}>
      <Link href="/explore?tab=profiles" className={styles.back}>&#x2190; explore</Link>

      <div className={styles.header}>
        <div className={styles.titleRow}>
          <h1 className={styles.name}>{data.name}</h1>
          <span className={styles.agentBadge}>
            {data.totalAgents} agent{data.totalAgents !== 1 ? "s" : ""}
          </span>
        </div>
        {data.description && (
          <p className={styles.description}>{data.description}</p>
        )}
        <p className={styles.kind}>
          {data.kind} · {data.llms.length} model{data.llms.length !== 1 ? "s" : ""}
        </p>
      </div>

      {data.weights.length > 0 && (
        <div className={styles.weightBar}>
          {data.weights.map((w, i) => {
            const ratio = weightMax > 0 ? w / weightMax : 0;
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

      <div className={styles.tiers}>
        <TierSection label="frontier" agents={tiers.frontier} labelClass={styles.tierLabelFrontier} />
        <TierSection label="mid" agents={tiers.mid} />
        <TierSection label="budget" agents={tiers.budget} />
      </div>
    </div>
  );
}

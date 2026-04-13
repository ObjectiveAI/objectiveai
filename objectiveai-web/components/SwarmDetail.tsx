"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import { apiFetch } from "@/lib/client";
import styles from "./SwarmDetail.module.css";

interface Props {
  id: string;
}

interface RawLlm {
  id: string;
  model: string;
  output_mode: string;
  top_logprobs?: number | null;
  temperature?: number | null;
  reasoning?: { enabled: boolean } | null;
  count?: number | null;
  fallbacks?: RawLlm[] | null;
}

interface SwarmResponse {
  id: string;
  created: number;
  llms: RawLlm[];
}

export function SwarmDetail({ id }: Props) {
  const [data, setData] = useState<SwarmResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    apiFetch<SwarmResponse>(`/ensembles/${id}`)
      .then((d) => {
        if (!cancelled) { setData(d); setLoading(false); }
      })
      .catch((err) => {
        if (!cancelled) { setError(err.message); setLoading(false); }
      });
    return () => { cancelled = true; };
  }, [id]);

  if (loading) {
    return (
      <div className={styles.loading} role="status" aria-live="polite">
        <span className={styles.loadingDot} />
        loading swarm
      </div>
    );
  }

  if (error || !data) {
    const msg = error?.includes("fetch") || error?.includes("network")
      ? "unable to reach api"
      : "unable to load swarm";
    return (
      <div className={styles.error} role="alert">
        <span>{msg}</span>
        <button
          onClick={() => window.location.reload()}
          className={styles.retryBtn}
        >
          try again
        </button>
      </div>
    );
  }

  const totalCount = data.llms.reduce((s, l) => s + (l.count ?? 1), 0);
  const uniqueModels = new Set(data.llms.map((l) => l.model)).size;
  const created = new Date(data.created * 1000);

  // Derive a readable label from models
  const models = data.llms.map((l) => shortModel(l.model));
  const label = models.length <= 3
    ? models.join(" + ")
    : `${models.slice(0, 2).join(" + ")} +${models.length - 2}`;

  return (
    <div className={styles.detail}>
      <Link href="/explore" className={styles.back}>&#x2190; explore</Link>

      <div className={styles.header}>
        <div className={styles.titleRow}>
          <h1 className={styles.name}>{label}</h1>
          <span className={styles.countBadge}>
            {totalCount} agent{totalCount !== 1 ? "s" : ""}
          </span>
        </div>
        <p className={styles.id}>{data.id}</p>
        <p className={styles.created}>
          created {created.toLocaleDateString()} · {uniqueModels} unique model{uniqueModels !== 1 ? "s" : ""}
        </p>
      </div>

      <div className={styles.agents}>
        {data.llms.map((llm) => (
          <div key={llm.id} className={styles.agentCard}>
            <div className={styles.agentMain}>
              <span className={styles.agentModel}>{llm.model}</span>
              <span className={styles.agentTags}>
                <span className={styles.tag}>{llm.output_mode}</span>
                {llm.top_logprobs != null && (
                  <span className={`${styles.tag} ${styles.tagCopper}`}>
                    logprobs:{llm.top_logprobs}
                  </span>
                )}
                {llm.temperature != null && (
                  <span className={styles.tag}>t:{llm.temperature}</span>
                )}
                {llm.reasoning?.enabled === false && (
                  <span className={styles.tag}>reasoning:off</span>
                )}
                {(llm.count ?? 1) > 1 && (
                  <span className={`${styles.tag} ${styles.tagCopper}`}>
                    ×{llm.count}
                  </span>
                )}
              </span>
            </div>
            {llm.fallbacks && llm.fallbacks.length > 0 && (
              <div className={styles.fallbacks}>
                <span className={styles.fallbackLabel}>
                  fallbacks ({llm.fallbacks.length})
                </span>
                {llm.fallbacks.map((fb) => (
                  <div key={fb.id} className={styles.fallback}>
                    <span className={styles.fallbackModel}>{fb.model}</span>
                    <span className={styles.fallbackMode}>{fb.output_mode}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function shortModel(model: string): string {
  return model.includes("/") ? model.split("/").pop()! : model;
}

"use client";

import { useState, useEffect, useMemo } from "react";
import type { SwarmMeta } from "@/lib/swarms/types";
import { fetchAllSwarms } from "@/lib/swarms/fetch";
import { SwarmCard } from "./SwarmCard";
import styles from "./SwarmCard.module.css";

export function SwarmsBrowse() {
  const [swarms, setSwarms] = useState<SwarmMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetchAllSwarms()
      .then((data) => {
        if (!cancelled) {
          setSwarms(data);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err.message);
          setLoading(false);
        }
      });
    return () => { cancelled = true; };
  }, []);

  // Sort by agent count descending (largest swarms first)
  const sorted = useMemo(() => {
    return [...swarms].sort((a, b) => b.totalAgentCount - a.totalAgentCount);
  }, [swarms]);

  if (loading) {
    return (
      <div className={styles.loading} role="status" aria-live="polite">
        <span className={styles.loadingDot} />
        loading swarms
      </div>
    );
  }

  if (error || swarms.length === 0) {
    return (
      <div className={styles.browse}>
        <div className={styles.pageHeader}>
          <h2 className={styles.pageTitle}>swarms</h2>
        </div>
        <div className={styles.loading}>
          swarms are created at execution time
        </div>
      </div>
    );
  }

  return (
    <div className={styles.browse}>
      <div className={styles.pageHeader}>
        <h1 className={styles.pageTitle}>swarms</h1>
        <span className={styles.pageCount}>{swarms.length}</span>
      </div>
      <div className={styles.grid}>
        {sorted.map((swarm) => (
          <SwarmCard key={swarm.id} swarm={swarm} />
        ))}
      </div>
    </div>
  );
}

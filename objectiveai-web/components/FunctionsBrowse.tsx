"use client";

import { useState, useEffect, useMemo } from "react";
import type { FunctionMeta } from "@/lib/functions/types";
import { fetchAllFunctions } from "@/lib/functions/fetch";
import { FunctionCard } from "./FunctionCard";
import styles from "./FunctionCard.module.css";

export function FunctionsBrowse() {
  const [functions, setFunctions] = useState<FunctionMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetchAllFunctions()
      .then((fns) => {
        if (!cancelled) {
          setFunctions(fns);
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

  // Sort: branch functions first (they compose others), then alphabetical
  const sorted = useMemo(() => {
    return [...functions].sort((a, b) => {
      const aIsBranch = a.subFunctions.length > 0 ? 0 : 1;
      const bIsBranch = b.subFunctions.length > 0 ? 0 : 1;
      if (aIsBranch !== bIsBranch) return aIsBranch - bIsBranch;
      return a.name.localeCompare(b.name);
    });
  }, [functions]);

  if (loading) {
    return (
      <div className={styles.loading} role="status" aria-live="polite">
        <span className={styles.loadingDot} />
        loading functions
      </div>
    );
  }

  if (error || functions.length === 0) {
    return (
      <div className={styles.browse}>
        <div className={styles.pageHeader}>
          <h1 className={styles.pageTitle}>functions</h1>
        </div>
        <div className={styles.loading}>
          functions are registered via the api and loaded from github
        </div>
      </div>
    );
  }

  return (
    <div className={styles.browse}>
      <div className={styles.pageHeader}>
        <h1 className={styles.pageTitle}>functions</h1>
        <span className={styles.pageCount}>{functions.length}</span>
      </div>
      <div className={styles.grid}>
        {sorted.map((fn) => (
          <FunctionCard key={`${fn.owner}/${fn.repository}`} fn={fn} />
        ))}
      </div>
    </div>
  );
}

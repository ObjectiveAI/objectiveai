"use client";

import { useState, useEffect } from "react";
import type { ProfileMeta } from "@/lib/profiles/types";
import { fetchDefaultProfiles } from "@/lib/profiles/fetch";
import { ProfileCard } from "./ProfileCard";
import styles from "./ProfileCard.module.css";

export function ProfilesBrowse() {
  const [profiles, setProfiles] = useState<ProfileMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetchDefaultProfiles()
      .then((data) => {
        if (!cancelled) {
          setProfiles(data);
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

  if (loading) {
    return (
      <div className={styles.loading} role="status" aria-live="polite">
        <span className={styles.loadingDot} />
        loading profiles
      </div>
    );
  }

  if (error) {
    return (
      <div className={styles.error} role="alert">
        <span>unable to load profiles</span>
        <button
          onClick={() => window.location.reload()}
          className={styles.retryBtn}
        >
          try again
        </button>
      </div>
    );
  }

  return (
    <div className={styles.browse}>
      <div className={styles.pageHeader}>
        <h1 className={styles.pageTitle}>profiles</h1>
        <span className={styles.pageCount}>5 default</span>
      </div>
      <div className={styles.showcaseGrid}>
        {profiles.map((p) => (
          <ProfileCard key={p.name} profile={p} />
        ))}
      </div>
    </div>
  );
}

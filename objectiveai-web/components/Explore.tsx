"use client";

import { useState, useEffect, useMemo } from "react";
import type { FunctionMeta } from "@/lib/functions/types";
import type { ProfileMeta } from "@/lib/profiles/types";
import type { SwarmMeta } from "@/lib/swarms/types";
import { fetchAllFunctions } from "@/lib/functions/fetch";
import { fetchDefaultProfiles } from "@/lib/profiles/fetch";
import { fetchAllSwarms } from "@/lib/swarms/fetch";
import { FunctionCard } from "./FunctionCard";
import { ProfileCard } from "./ProfileCard";
import { SwarmCard } from "./SwarmCard";
import styles from "./Explore.module.css";

export function Explore() {
  const [functions, setFunctions] = useState<FunctionMeta[]>([]);
  const [profiles, setProfiles] = useState<ProfileMeta[]>([]);
  const [swarms, setSwarms] = useState<SwarmMeta[]>([]);
  const [loadingFn, setLoadingFn] = useState(true);
  const [loadingPr, setLoadingPr] = useState(true);
  const [loadingSw, setLoadingSw] = useState(true);
  const [errorFn, setErrorFn] = useState<string | null>(null);
  const [errorPr, setErrorPr] = useState<string | null>(null);
  const [errorSw, setErrorSw] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    fetchAllFunctions()
      .then((fns) => { if (!cancelled) { setFunctions(fns); setLoadingFn(false); } })
      .catch((err) => { if (!cancelled) { setErrorFn(err instanceof Error ? err.message : "failed to load"); setLoadingFn(false); } });

    fetchDefaultProfiles()
      .then((prs) => { if (!cancelled) { setProfiles(prs); setLoadingPr(false); } })
      .catch((err) => { if (!cancelled) { setErrorPr(err instanceof Error ? err.message : "failed to load"); setLoadingPr(false); } });

    fetchAllSwarms()
      .then((sws) => { if (!cancelled) { setSwarms(sws); setLoadingSw(false); } })
      .catch((err) => { if (!cancelled) { setErrorSw(err instanceof Error ? err.message : "failed to load"); setLoadingSw(false); } });

    return () => { cancelled = true; };
  }, []);

  const sortedFunctions = useMemo(() => {
    return [...functions].sort((a, b) => {
      const aIsBranch = a.subFunctions.length > 0 ? 0 : 1;
      const bIsBranch = b.subFunctions.length > 0 ? 0 : 1;
      if (aIsBranch !== bIsBranch) return aIsBranch - bIsBranch;
      return a.name.localeCompare(b.name);
    });
  }, [functions]);

  const sortedSwarms = useMemo(() => {
    return [...swarms].sort((a, b) => b.totalAgentCount - a.totalAgentCount);
  }, [swarms]);

  return (
    <div className={styles.page}>
      <div className={styles.pageHeader}>
        <h1 className={styles.pageTitle}>explore</h1>
        <p className={styles.pageSubtitle}>
          Lorem ipsum dolor sit amet — functions, profiles, and swarms registered on the network.
        </p>
      </div>

      {/* ── Functions ── */}
      <section className={styles.section}>
        <div className={styles.sectionHeader}>
          <h2 className={styles.sectionTitle}>functions</h2>
          {!loadingFn && functions.length > 0 && (
            <span className={styles.sectionCount}>{functions.length}</span>
          )}
        </div>
        {loadingFn ? (
          <div className={styles.sectionLoading}>
            <span className={styles.sectionLoadingDot} />
            loading functions
          </div>
        ) : errorFn ? (
          <div className={styles.sectionError}>
            could not load functions &mdash; {errorFn}
          </div>
        ) : functions.length === 0 ? (
          <div className={styles.sectionEmpty}>
            no functions registered yet
          </div>
        ) : (
          <div className={styles.functionsGrid}>
            {sortedFunctions.map((fn) => (
              <FunctionCard key={`${fn.owner}/${fn.repository}`} fn={fn} />
            ))}
          </div>
        )}
      </section>

      {/* ── Profiles ── */}
      <section className={styles.section}>
        <div className={styles.sectionHeader}>
          <h2 className={styles.sectionTitle}>profiles</h2>
          {!loadingPr && profiles.length > 0 && (
            <span className={styles.sectionCount}>{profiles.length}</span>
          )}
        </div>
        {loadingPr ? (
          <div className={styles.sectionLoading}>
            <span className={styles.sectionLoadingDot} />
            loading profiles
          </div>
        ) : errorPr ? (
          <div className={styles.sectionError}>
            could not load profiles &mdash; {errorPr}
          </div>
        ) : profiles.length === 0 ? (
          <div className={styles.sectionEmpty}>
            no profiles available
          </div>
        ) : (
          <div className={styles.profilesGrid}>
            {profiles.map((p) => (
              <ProfileCard key={p.name} profile={p} />
            ))}
          </div>
        )}
      </section>

      {/* ── Swarms ── */}
      <section className={styles.section}>
        <div className={styles.sectionHeader}>
          <h2 className={styles.sectionTitle}>swarms</h2>
          {!loadingSw && swarms.length > 0 && (
            <span className={styles.sectionCount}>{swarms.length}</span>
          )}
        </div>
        {loadingSw ? (
          <div className={styles.sectionLoading}>
            <span className={styles.sectionLoadingDot} />
            loading swarms
          </div>
        ) : errorSw ? (
          <div className={styles.sectionError}>
            could not load swarms &mdash; {errorSw}
          </div>
        ) : swarms.length === 0 ? (
          <div className={styles.sectionEmpty}>
            no swarms created yet
          </div>
        ) : (
          <div className={styles.swarmsGrid}>
            {sortedSwarms.map((swarm) => (
              <SwarmCard key={swarm.id} swarm={swarm} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

"use client";

import { useState, useEffect, useMemo, useRef, useCallback } from "react";
import type { FunctionMeta } from "@/lib/functions/types";
import type { ProfileMeta } from "@/lib/profiles/types";
import type { SwarmMeta } from "@/lib/swarms/types";
// TEMPORARY — using mock data for density testing. Revert these 3 imports when done:
// import { fetchAllFunctions } from "@/lib/functions/fetch";
// import { fetchDefaultProfiles } from "@/lib/profiles/fetch";
// import { fetchAllSwarms } from "@/lib/swarms/fetch";
import { MOCK_FUNCTIONS, MOCK_PROFILES, MOCK_SWARMS } from "@/lib/TEMPORARY_mock_explore";
import { FunctionCard } from "./FunctionCard";
import { ProfileCard } from "./ProfileCard";
import { SwarmCard } from "./SwarmCard";
import styles from "./Explore.module.css";

const SECTIONS = ["functions", "profiles", "swarms"] as const;
type SectionKey = (typeof SECTIONS)[number];

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
  const [activeSection, setActiveSection] = useState<SectionKey>("functions");

  const sectionRefs = useRef<Record<SectionKey, HTMLElement | null>>({
    functions: null,
    profiles: null,
    swarms: null,
  });

  const scrollToSection = useCallback((key: SectionKey) => {
    sectionRefs.current[key]?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, []);

  useEffect(() => {
    const observers: IntersectionObserver[] = [];
    for (const key of SECTIONS) {
      const el = sectionRefs.current[key];
      if (!el) continue;
      const obs = new IntersectionObserver(
        ([entry]) => { if (entry.isIntersecting) setActiveSection(key); },
        { rootMargin: "-80px 0px -60% 0px", threshold: 0 },
      );
      obs.observe(el);
      observers.push(obs);
    }
    return () => observers.forEach((o) => o.disconnect());
  }, []);

  // TEMPORARY — mock data for density testing. Restore the real fetches when done.
  useEffect(() => {
    setFunctions(MOCK_FUNCTIONS);
    setLoadingFn(false);
    setProfiles(MOCK_PROFILES);
    setLoadingPr(false);
    setSwarms(MOCK_SWARMS);
    setLoadingSw(false);
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
      </div>

      <nav className={styles.tabBar}>
        {SECTIONS.map((key) => (
          <button
            key={key}
            className={`${styles.tab} ${activeSection === key ? styles.tabActive : ""}`}
            onClick={() => scrollToSection(key)}
          >
            {key}
          </button>
        ))}
      </nav>

      {/* ── Functions ── */}
      <section className={styles.section} ref={(el) => { sectionRefs.current.functions = el; }}>
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
      <section className={styles.section} ref={(el) => { sectionRefs.current.profiles = el; }}>
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
      <section className={styles.section} ref={(el) => { sectionRefs.current.swarms = el; }}>
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

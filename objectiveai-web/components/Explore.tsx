"use client";

import { useState, useEffect, useMemo, useCallback, Suspense } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import type { FunctionMeta } from "@/lib/functions/types";
import type { ProfileMeta } from "@/lib/profiles/types";
import type { SwarmMeta } from "@/lib/swarms/types";
import { MOCK_FUNCTIONS, MOCK_PROFILES, MOCK_SWARMS } from "@/lib/TEMPORARY_mock_explore";
import { FunctionCard } from "./FunctionCard";
import { ProfileCard } from "./ProfileCard";
import { SwarmCard } from "./SwarmCard";
import { ExploreToolbar, type FilterDef } from "./ExploreToolbar";
import styles from "./Explore.module.css";

const TABS = ["functions", "profiles", "swarms"] as const;
type TabKey = (typeof TABS)[number];

const FUNCTION_FILTERS: FilterDef[] = [
  {
    key: "category",
    label: "type",
    options: [
      { value: "all", label: "all" },
      { value: "scalar", label: "scalar" },
      { value: "vector", label: "vector" },
    ],
  },
  {
    key: "depth",
    label: "depth",
    options: [
      { value: "all", label: "all" },
      { value: "branch", label: "branch" },
      { value: "leaf", label: "leaf" },
    ],
  },
];

const PROFILE_FILTERS: FilterDef[] = [
  {
    key: "tier",
    label: "tier",
    options: [
      { value: "all", label: "all" },
      { value: "frontier", label: "has frontier" },
      { value: "budget", label: "budget only" },
    ],
  },
];

function ExploreInner() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const tab = (searchParams.get("tab") as TabKey) || "functions";

  const [functions, setFunctions] = useState<FunctionMeta[]>([]);
  const [profiles, setProfiles] = useState<ProfileMeta[]>([]);
  const [swarms, setSwarms] = useState<SwarmMeta[]>([]);
  const [loading, setLoading] = useState(true);

  const [search, setSearch] = useState("");
  const [fnFilters, setFnFilters] = useState<Record<string, string>>({});
  const [prFilters, setPrFilters] = useState<Record<string, string>>({});
  const [swFilters, setSwFilters] = useState<Record<string, string>>({});

  const setTab = useCallback(
    (key: TabKey) => {
      router.push(`/explore?tab=${key}`, { scroll: false });
      setSearch("");
    },
    [router],
  );

  useEffect(() => {
    setFunctions(MOCK_FUNCTIONS);
    setProfiles(MOCK_PROFILES);
    setSwarms(MOCK_SWARMS);
    setLoading(false);
  }, []);

  const filteredFunctions = useMemo(() => {
    let result = [...functions];
    const q = search.toLowerCase();
    if (q) {
      result = result.filter(
        (fn) => fn.name.toLowerCase().includes(q) || fn.description.toLowerCase().includes(q),
      );
    }
    const cat = fnFilters.category;
    if (cat && cat !== "all") {
      result = result.filter((fn) => fn.category === cat);
    }
    const depth = fnFilters.depth;
    if (depth && depth !== "all") {
      result = result.filter((fn) => fn.depth === depth);
    }
    return result.sort((a, b) => {
      const aB = a.subFunctions.length > 0 ? 0 : 1;
      const bB = b.subFunctions.length > 0 ? 0 : 1;
      if (aB !== bB) return aB - bB;
      return a.name.localeCompare(b.name);
    });
  }, [functions, search, fnFilters]);

  const filteredProfiles = useMemo(() => {
    let result = [...profiles];
    const q = search.toLowerCase();
    if (q) {
      result = result.filter(
        (p) => p.name.toLowerCase().includes(q) || p.description.toLowerCase().includes(q),
      );
    }
    const tier = prFilters.tier;
    if (tier === "frontier") {
      result = result.filter((p) => p.tiers.frontier.length > 0);
    } else if (tier === "budget") {
      result = result.filter((p) => p.tiers.frontier.length === 0 && p.tiers.mid.length === 0);
    }
    return result;
  }, [profiles, search, prFilters]);

  const swarmModelSet = useMemo(() => {
    const models = new Set<string>();
    swarms.forEach((s) => s.agents.forEach((a) => {
      const short = a.model.includes("/") ? a.model.split("/").pop()! : a.model;
      models.add(short);
    }));
    return Array.from(models).sort();
  }, [swarms]);

  const swarmFilters = useMemo<FilterDef[]>(() => {
    if (swarmModelSet.length === 0) return [];
    return [
      {
        key: "model",
        label: "model",
        options: [
          { value: "all", label: "all" },
          ...swarmModelSet.slice(0, 6).map((m) => ({ value: m, label: m })),
        ],
      },
    ];
  }, [swarmModelSet]);

  const filteredSwarms = useMemo(() => {
    let result = [...swarms];
    const q = search.toLowerCase();
    if (q) {
      result = result.filter((s) =>
        s.agents.some((a) => a.model.toLowerCase().includes(q)),
      );
    }
    const model = swFilters.model;
    if (model && model !== "all") {
      result = result.filter((s) =>
        s.agents.some((a) => {
          const short = a.model.includes("/") ? a.model.split("/").pop()! : a.model;
          return short === model;
        }),
      );
    }
    return result.sort((a, b) => b.totalAgentCount - a.totalAgentCount);
  }, [swarms, search, swFilters]);

  const counts: Record<TabKey, { filtered: number; total: number }> = {
    functions: { filtered: filteredFunctions.length, total: functions.length },
    profiles: { filtered: filteredProfiles.length, total: profiles.length },
    swarms: { filtered: filteredSwarms.length, total: swarms.length },
  };

  const activeFilters = tab === "functions" ? fnFilters : tab === "profiles" ? prFilters : swFilters;
  const setActiveFilters = tab === "functions"
    ? (k: string, v: string) => setFnFilters((f) => ({ ...f, [k]: v }))
    : tab === "profiles"
      ? (k: string, v: string) => setPrFilters((f) => ({ ...f, [k]: v }))
      : (k: string, v: string) => setSwFilters((f) => ({ ...f, [k]: v }));

  const filters = tab === "functions" ? FUNCTION_FILTERS : tab === "profiles" ? PROFILE_FILTERS : swarmFilters;

  return (
    <div className={styles.page}>
      <nav className={styles.tabBar}>
        {TABS.map((key) => (
          <button
            key={key}
            className={`${styles.tab} ${tab === key ? styles.tabActive : ""}`}
            onClick={() => setTab(key)}
          >
            {key}
            {!loading && (
              <span className={styles.tabCount}>{counts[key].total}</span>
            )}
          </button>
        ))}
      </nav>

      <ExploreToolbar
        search={search}
        onSearchChange={setSearch}
        filters={filters}
        activeFilters={activeFilters}
        onFilterChange={setActiveFilters}
        resultCount={counts[tab].filtered}
        totalCount={counts[tab].total}
        placeholder={`search ${tab}...`}
      />

      <div className={styles.grid}>
        {loading ? (
          <div className={styles.loading}>
            <span className={styles.loadingDot} />
            loading {tab}
          </div>
        ) : tab === "functions" ? (
          filteredFunctions.length === 0 ? (
            <div className={styles.empty}>no functions match</div>
          ) : (
            <div className={styles.functionsGrid}>
              {filteredFunctions.map((fn) => (
                <FunctionCard key={`${fn.owner}/${fn.repository}`} fn={fn} />
              ))}
            </div>
          )
        ) : tab === "profiles" ? (
          filteredProfiles.length === 0 ? (
            <div className={styles.empty}>no profiles match</div>
          ) : (
            <div className={styles.profilesGrid}>
              {filteredProfiles.map((p) => (
                <ProfileCard key={p.name} profile={p} />
              ))}
            </div>
          )
        ) : (
          filteredSwarms.length === 0 ? (
            <div className={styles.empty}>no swarms match</div>
          ) : (
            <div className={styles.swarmsGrid}>
              {filteredSwarms.map((swarm) => (
                <SwarmCard key={swarm.id} swarm={swarm} />
              ))}
            </div>
          )
        )}
      </div>
    </div>
  );
}

export function Explore() {
  return (
    <Suspense>
      <ExploreInner />
    </Suspense>
  );
}

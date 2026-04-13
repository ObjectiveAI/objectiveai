"use client";

import { useState, useEffect, useCallback } from "react";
import Link from "next/link";
import type { FunctionDefinition } from "@/lib/functions/types";
import { fetchFunctionList, fetchRecursive } from "@/lib/functions/fetch";
import { apiFetch } from "@/lib/client";
import { fetchProfileBySlug } from "@/lib/profiles/fetch";
import type { ProfileMeta } from "@/lib/profiles/types";
import { JudgmentStack } from "./JudgmentStack";
import type { FunctionExecution } from "./JudgmentStack";
import { SchemaForm } from "./SchemaForm";
import { DEMO_EXECUTION_COMPLETE, DEMO_MODEL_NAMES } from "@/lib/demo-data";
import styles from "./FunctionCard.module.css";

interface Props {
  owner: string;
  repo: string;
}

type DefMap = Map<string, FunctionDefinition>;

export function FunctionDetail({ owner, repo }: Props) {
  const [rootDef, setRootDef] = useState<FunctionDefinition | null>(null);
  const [allDefs, setAllDefs] = useState<DefMap>(new Map());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [schemaOpen, setSchemaOpen] = useState(false);
  const [pairedProfile, setPairedProfile] = useState<ProfileMeta | null>(null);
  const [executeOpen, setExecuteOpen] = useState(false);
  const [execution, setExecution] = useState<FunctionExecution | null>(null);
  const [execState, setExecState] = useState<"idle" | "streaming" | "done">("idle");

  const handleExecute = useCallback((_values: Record<string, unknown>) => {
    // TODO: Replace with useExecution() when API is available
    setExecState("streaming");
    // Simulate streaming delay then show full execution
    const t = setTimeout(() => {
      setExecution(DEMO_EXECUTION_COMPLETE);
      setExecState("done");
    }, 800);
    return () => clearTimeout(t);
  }, []);

  useEffect(() => {
    let cancelled = false;

    // Fetch paired profile — try pairs endpoint, fall back to profile-standard
    apiFetch<{ data: Array<{ function: { owner: string; repository: string }; profile: { owner: string; repository: string } }> }>("/functions/profiles/pairs")
      .then(async (pairs) => {
        if (cancelled) return;
        const match = pairs.data.find((p) => p.function.owner === owner && p.function.repository === repo);
        if (match) {
          const full = await fetchProfileBySlug(match.profile.owner, match.profile.repository);
          if (!cancelled) setPairedProfile(full);
        } else {
          const fallback = await fetchProfileBySlug("ObjectiveAI", "profile-standard");
          if (!cancelled) setPairedProfile(fallback);
        }
      })
      .catch(async () => {
        try {
          const fallback = await fetchProfileBySlug("ObjectiveAI", "profile-standard");
          if (!cancelled) setPairedProfile(fallback);
        } catch { /* truly offline */ }
      });

    fetchFunctionList()
      .then(async (items) => {
        const item = items.find(
          (f) => f.owner === owner && f.repository === repo
        );
        if (!item) throw new Error("Function not found");

        const commitMap = new Map<string, string>();
        for (const i of items) {
          commitMap.set(`${i.owner}/${i.repository}`, i.commit);
        }

        const defs = new Map<string, FunctionDefinition>();
        await fetchRecursive(item.owner, item.repository, item.commit, defs, commitMap);

        const root = defs.get(`${item.owner}/${item.repository}`);
        if (!root) throw new Error("Failed to fetch root definition");

        if (!cancelled) {
          setRootDef(root);
          setAllDefs(defs);
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
  }, [owner, repo]);

  if (loading) {
    return (
      <div className={styles.loading} role="status" aria-live="polite">
        <span className={styles.loadingDot} />
        loading {repo}
      </div>
    );
  }

  if (error) {
    return <div className={styles.error} role="alert">unable to load function</div>;
  }

  if (!rootDef) return null;

  const type = rootDef.type
    .replace(/^alpha\./, "")
    .replace(/\.function$/, "");

  const schema = rootDef.input_schema;
  const properties = (schema?.properties ?? {}) as Record<string, Record<string, unknown>>;
  const required = (schema?.required ?? []) as string[];
  const hasSchema = Object.keys(properties).length > 0;

  return (
    <div className={styles.detail}>
      <div className={styles.detailHeader}>
        <Link href="/explore" className={styles.backLink}>
          &#x2190; explore
        </Link>
        <div className={styles.detailTitleRow}>
          <h1 className={styles.detailName}>{repo}</h1>
          <span className={styles.detailType}>{type}</span>
        </div>
        {rootDef.description && (
          <p className={styles.detailDescription}>{rootDef.description}</p>
        )}
        <p className={styles.detailOwner}>{owner}</p>

        {pairedProfile && (
          <div className={styles.profileCard}>
            <div className={styles.profileCardHeader}>
              <Link href="/profiles" className={styles.profileCardName}>
                {pairedProfile.name}
              </Link>
              <span className={styles.profileCardBadge}>{pairedProfile.totalAgents} agents</span>
            </div>
            {pairedProfile.description && (
              <p className={styles.profileCardDesc}>{pairedProfile.description}</p>
            )}
            {pairedProfile.weights.length > 0 && (
              <div className={styles.profileCardWeightBar}>
                {pairedProfile.weights.map((w, i) => {
                  const maxW = Math.max(...pairedProfile.weights);
                  const ratio = maxW > 0 ? w / maxW : 0;
                  return (
                    <div
                      key={i}
                      className={styles.profileCardWeightSeg}
                      style={{ flex: Math.max(w, 0.01), opacity: 0.2 + ratio * 0.8 }}
                    />
                  );
                })}
              </div>
            )}
            <div className={styles.profileCardTiers}>
              {pairedProfile.tiers.frontier.length > 0 && (
                <span className={styles.profileCardTier}>
                  <span className={styles.profileCardTierDot} data-tier="frontier" />
                  {pairedProfile.tiers.frontier.length} frontier
                </span>
              )}
              {pairedProfile.tiers.mid.length > 0 && (
                <span className={styles.profileCardTier}>
                  <span className={styles.profileCardTierDot} data-tier="mid" />
                  {pairedProfile.tiers.mid.length} mid
                </span>
              )}
              {pairedProfile.tiers.budget.length > 0 && (
                <span className={styles.profileCardTier}>
                  <span className={styles.profileCardTierDot} data-tier="budget" />
                  {pairedProfile.tiers.budget.length} budget
                </span>
              )}
            </div>
          </div>
        )}

        {hasSchema && (
          <div className={styles.schema}>
            <button
              className={styles.schemaToggle}
              onClick={() => setSchemaOpen(!schemaOpen)}
            >
              <span className={`${styles.schemaArrow} ${schemaOpen ? styles.schemaArrowOpen : ""}`}>
                &#x25B8;
              </span>
              input_schema
              {typeof schema?.description === "string" && (
                <span style={{ color: "var(--info-dim)", fontWeight: 400 }}>
                  &mdash; {schema.description}
                </span>
              )}
            </button>
            {schemaOpen && (
              <div className={styles.schemaBody}>
                {Object.entries(properties).map(([key, prop]) => (
                  <div key={key} className={styles.schemaProperty}>
                    <span className={styles.schemaKey}>{key}</span>
                    <span className={styles.schemaType}>
                      {renderSchemaType(prop)}
                    </span>
                    {required.includes(key) && (
                      <span className={styles.schemaRequired}>required</span>
                    )}
                    {typeof prop.description === "string" && (
                      <span className={styles.schemaDesc}>
                        {prop.description}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
        {hasSchema && (
          <div className={styles.executePanel}>
            <button
              className={styles.executeToggle}
              onClick={() => setExecuteOpen(!executeOpen)}
            >
              <span className={`${styles.schemaArrow} ${executeOpen ? styles.schemaArrowOpen : ""}`}>
                &#x25B8;
              </span>
              execute
            </button>
            {executeOpen && (
              <div className={styles.executeBody}>
                <SchemaForm
                  schema={schema as { type?: string; properties?: Record<string, Record<string, unknown>>; required?: string[]; description?: string }}
                  onSubmit={handleExecute}
                  disabled={execState === "streaming"}
                />
                {execState !== "idle" && (
                  <div className={styles.executeState}>
                    <span
                      className={styles.executeStateDot}
                      style={{
                        background: execState === "streaming" ? "var(--copper-mid)" : "var(--copper-hot)",
                      }}
                    />
                    {execState === "streaming" ? "executing..." : "complete"}
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      <JudgmentStack
        definition={rootDef}
        execution={execution}
        profile={pairedProfile}
        resolvedSubFunctions={allDefs}
        modelNames={execution ? DEMO_MODEL_NAMES : undefined}
      />
    </div>
  );
}

function renderSchemaType(prop: Record<string, unknown>): string {
  if (prop.anyOf && Array.isArray(prop.anyOf)) {
    return (prop.anyOf as Record<string, unknown>[])
      .map((v) => (v.type as string) ?? "unknown")
      .join(" | ");
  }
  return (prop.type as string) ?? "object";
}

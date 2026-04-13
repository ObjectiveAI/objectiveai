"use client";

import { useState, useEffect, useMemo, type FormEvent } from "react";
import Link from "next/link";
import type { FunctionMeta } from "@/lib/functions/types";
import type { SwarmMeta } from "@/lib/swarms/types";
import type { ProfileMeta } from "@/lib/profiles/types";
import { fetchAllFunctions } from "@/lib/functions/fetch";
import { fetchAllSwarms } from "@/lib/swarms/fetch";
import { fetchDefaultProfiles } from "@/lib/profiles/fetch";
import styles from "./Landing.module.css";

/* ── Logo components ── */

function LogoMark({ className }: { className?: string }) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="68 97 188 130" fill="currentColor" className={className}>
      <path d="M75.01,168.97h-3.01v-13.94h3.01c6.02,0,10.94-4.92,10.94-10.94v-17.64c0-13.67,11.21-24.88,24.88-24.88h7.66v13.94h-7.66c-6.02,0-10.94,4.92-10.94,10.94v17.64c0,6.97-3.01,13.4-7.66,17.91,4.65,4.51,7.66,10.94,7.66,17.91v17.64c0,6.01,4.92,10.94,10.94,10.94h7.66v13.94h-7.66c-13.67,0-24.88-11.21-24.88-24.88v-17.64c0-6.02-4.92-10.94-10.94-10.94Z"/>
      <path d="M231.77,162c-4.65-4.51-7.66-10.94-7.66-17.91v-17.64c0-6.02-4.92-10.94-10.94-10.94h-7.66v-13.94h7.66c13.67,0,24.88,11.21,24.88,24.88v17.64c0,6.02,4.92,10.94,10.94,10.94h3.01v13.94h-3.01c-6.02,0-10.94,4.92-10.94,10.94v17.64c0,13.67-11.21,24.88-24.88,24.88h-7.66v-13.94h7.66c6.02,0,10.94-4.92,10.94-10.94v-17.64c0-6.97,3.01-13.4,7.66-17.91Z"/>
      <path d="M123.38,155.18c.42-.49,1.29-1.22,2.59-2.17,1.3-.95,2.96-1.9,4.97-2.86,2.01-.95,4.36-1.78,7.04-2.49,2.68-.7,5.61-1.06,8.78-1.06s6.37.37,9.37,1.11c3,.74,5.66,1.96,7.99,3.65,2.33,1.69,4.18,3.91,5.56,6.67s2.06,6.14,2.06,10.16v31.43h-14.82l-1.27-6.03c-1.62,2.26-3.67,4.01-6.14,5.24-2.47,1.23-5.54,1.85-9.21,1.85-2.82,0-5.31-.41-7.46-1.22-2.15-.81-3.95-1.92-5.4-3.33-1.45-1.41-2.54-3.07-3.28-4.97-.74-1.91-1.11-3.95-1.11-6.14s.48-4.41,1.43-6.46c.95-2.04,2.47-3.83,4.55-5.34,2.08-1.52,4.78-2.73,8.1-3.65,3.32-.92,7.34-1.38,12.06-1.38h6.14v-.42c0-2.12-.76-3.77-2.28-4.98-1.52-1.2-4-1.8-7.46-1.8-1.55,0-3.05.21-4.5.64-1.45.42-2.75.92-3.92,1.48-1.16.57-2.17,1.15-3.02,1.75-.85.6-1.45,1.08-1.8,1.43l-8.99-11.11ZM155.34,177.4h-5.5c-3.53,0-5.96.65-7.3,1.96-1.34,1.31-2.01,2.77-2.01,4.39,0,1.2.46,2.33,1.38,3.39.92,1.06,2.4,1.59,4.45,1.59.85,0,1.8-.21,2.86-.63,1.06-.42,2.03-1.06,2.91-1.91.88-.85,1.64-1.92,2.28-3.23.63-1.3.95-2.8.95-4.5v-1.06Z"/>
      <path d="M178.94,133.8c0-1.55.28-3,.85-4.34.56-1.34,1.32-2.5,2.28-3.49.95-.99,2.08-1.76,3.39-2.33,1.3-.56,2.7-.85,4.18-.85s2.87.28,4.18.85c1.3.57,2.47,1.34,3.49,2.33,1.02.99,1.82,2.15,2.38,3.49.56,1.34.85,2.79.85,4.34s-.28,2.91-.85,4.29c-.57,1.38-1.36,2.56-2.38,3.54-1.02.99-2.19,1.78-3.49,2.38-1.31.6-2.7.9-4.18.9s-2.88-.3-4.18-.9c-1.31-.6-2.43-1.39-3.39-2.38-.95-.99-1.71-2.17-2.28-3.54-.57-1.38-.85-2.8-.85-4.29ZM180.95,147.77h17.46v51.86h-17.46v-51.86Z"/>
    </svg>
  );
}

/** Static example — realistic votes from a startup-idea-ranker execution */
const EXAMPLE_VOTES = [
  { model: "claude-haiku-4.5",        vote: [0.00, 1.00], weight: 1.0 },
  { model: "gpt-4.1-nano",            vote: [0.85, 0.15], weight: 1.0 },
  { model: "gpt-4.1-nano",            vote: [0.70, 0.30], weight: 1.0 },
  { model: "gemini-2.5-flash-lite",   vote: [0.60, 0.40], weight: 0.8 },
  { model: "gemini-2.5-flash-lite",   vote: [0.45, 0.55], weight: 0.8 },
  { model: "gemini-3-flash-preview",  vote: [0.55, 0.45], weight: 0.8 },
  { model: "gpt-5-mini",              vote: [0.90, 0.10], weight: 1.0 },
  { model: "grok-4.1-fast",           vote: [0.40, 0.60], weight: 0.5 },
  { model: "grok-4.1-fast",           vote: [0.50, 0.50], weight: 0.5 },
  { model: "deepseek-v3.2",           vote: [0.75, 0.25], weight: 0.3 },
  { model: "deepseek-v3.2",           vote: [0.60, 0.40], weight: 0.3 },
  { model: "gpt-4o-mini",             vote: [0.55, 0.45], weight: 0.5 },
];

export function Landing() {
  const [functions, setFunctions] = useState<FunctionMeta[]>([]);
  const [swarms, setSwarms] = useState<SwarmMeta[]>([]);
  const [profiles, setProfiles] = useState<ProfileMeta[]>([]);
  const [loaded, setLoaded] = useState({ fn: false, sw: false, pr: false });
  const [errors, setErrors] = useState({ fn: false, sw: false, pr: false });

  // Email signup state
  const [email, setEmail] = useState("");
  const [emailState, setEmailState] = useState<"idle" | "submitting" | "success" | "error">("idle");
  const [errorMsg, setErrorMsg] = useState("something went wrong");

  useEffect(() => {
    let cancelled = false;
    fetchAllFunctions()
      .then((fns) => { if (!cancelled) { setFunctions(fns); setLoaded((p) => ({ ...p, fn: true })); } })
      .catch(() => { if (!cancelled) { setErrors((p) => ({ ...p, fn: true })); setLoaded((p) => ({ ...p, fn: true })); } });
    fetchAllSwarms()
      .then((s) => { if (!cancelled) { setSwarms(s); setLoaded((p) => ({ ...p, sw: true })); } })
      .catch(() => { if (!cancelled) { setErrors((p) => ({ ...p, sw: true })); setLoaded((p) => ({ ...p, sw: true })); } });
    fetchDefaultProfiles()
      .then((p) => { if (!cancelled) { setProfiles(p); setLoaded((prev) => ({ ...prev, pr: true })); } })
      .catch(() => { if (!cancelled) { setErrors((p) => ({ ...p, pr: true })); setLoaded((p) => ({ ...p, pr: true })); } });
    return () => { cancelled = true; };
  }, []);

  const sortedFunctions = useMemo(() => {
    return [...functions].sort((a, b) => {
      const aB = a.subFunctions.length > 0 ? 0 : 1;
      const bB = b.subFunctions.length > 0 ? 0 : 1;
      if (aB !== bB) return aB - bB;
      return a.name.localeCompare(b.name);
    });
  }, [functions]);

  const sortedSwarms = useMemo(() => {
    return [...swarms].sort((a, b) => b.totalAgentCount - a.totalAgentCount);
  }, [swarms]);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (emailState === "submitting") return;
    if (!email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      setEmailState("error");
      setErrorMsg("enter a valid email");
      return;
    }
    setEmailState("submitting");
    try {
      const res = await fetch(
        "https://buttondown.com/api/emails/embed-subscribe/objectiveai",
        {
          method: "POST",
          headers: { "Content-Type": "application/x-www-form-urlencoded" },
          body: new URLSearchParams({ email }),
        }
      );
      if (res.ok || res.status === 303) {
        setEmailState("success");
        setEmail("");
      } else {
        setErrorMsg("something went wrong");
        setEmailState("error");
      }
    } catch {
      setErrorMsg("something went wrong");
      setEmailState("error");
    }
  }

  return (
    <div className={styles.landing}>

      {/* ── Hero ── */}
      <section className={styles.hero}>
        <div className={styles.heroBrand}>
          <LogoMark className={styles.logoMark} />
        </div>

        <h1 className={styles.heroTitle}>the agentic collective judgment harness</h1>

        {/* CONTENT: One-liner value prop.
            Type: What ObjectiveAI is in one sentence.
            Tone: Technical, matter-of-fact. Infrastructure language.
            Current text is real — refine with Maya. */}
        <p className={styles.heroBody}>
          Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod
          tempor incididunt ut labore et dolore magna aliqua.
        </p>

        <div className={styles.install}>
          <span className={styles.installCmd}>npm install objectiveai</span>
          <span className={styles.installCmd}>pip install objectiveai</span>
        </div>

        <div className={styles.emailSection}>
          <div className={styles.status}>shipping soon</div>
          <form className={styles.form} onSubmit={handleSubmit} noValidate>
            <input
              type="email"
              className={styles.emailInput}
              placeholder="your email"
              value={email}
              onChange={(e) => { setEmail(e.target.value); if (emailState === "error") setEmailState("idle"); }}
              required
            />
            <button type="submit" className={styles.submitBtn} disabled={emailState === "submitting"}>
              {emailState === "submitting" ? "..." : "notify me"}
            </button>
          </form>
          {emailState === "success" && <p className={styles.confirmation}>you&apos;re on the list</p>}
          {emailState === "error" && <p className={styles.error}>{errorMsg}</p>}
        </div>
      </section>

      {/* ── Execution comparison — one agent vs the swarm ── */}
      {/* CONTENT: This section is the visual hook.
          Type: Live demonstration of the core concept.
          Shows why collective judgment > single model.
          Keep as-is — it's the best explainer we have. */}
      <section className={styles.execution}>
        <p className={styles.bridging}>your agent doesn&apos;t have to decide alone</p>

        <div className={styles.executionHeader}>
          <span className={styles.executionLabel}>example execution</span>
          <span className={styles.executionFunction}>startup-idea-ranker</span>
        </div>
        <div className={styles.responseOptions}>
          <span className={styles.responseOption}>
            <span className={styles.responseMarker} data-option="a" />A: AI tutoring platform
          </span>
          <span className={styles.responseOption}>
            <span className={styles.responseMarker} data-option="b" />B: blockchain pet insurance
          </span>
        </div>

        <div className={styles.comparisonPanels}>
          {/* Left — single agent */}
          <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <span className={styles.panelTitle}>one agent</span>
            </div>
            <div className={styles.voteTable}>
              <div className={styles.voteTableHeader}>
                <span className={styles.voteTableCol}>agent</span>
                <span className={styles.voteTableColBar}>vote</span>
                <span className={styles.voteTableCol}>weight</span>
              </div>
              <div className={styles.agentRow}>
                <span className={styles.agentModel}>{EXAMPLE_VOTES[0].model}</span>
                <div className={styles.agentVoteBar}>
                  <div className={styles.voteSegmentA} style={{ flex: Math.max(EXAMPLE_VOTES[0].vote[0], 0.02) }} />
                  <div className={styles.voteSegmentB} style={{ flex: Math.max(EXAMPLE_VOTES[0].vote[1], 0.02) }} />
                </div>
                <span className={styles.agentWeight}>{EXAMPLE_VOTES[0].weight.toFixed(1)}</span>
              </div>
            </div>
            <div className={styles.scoresRow}>
              <span className={styles.scoresLabel}>scores</span>
              <div className={styles.scoreBar}>
                <div className={styles.scoreSegmentA} style={{ flex: 0.02 }} />
                <div className={styles.scoreSegmentB} style={{ flex: 0.98 }} />
              </div>
              <span className={styles.scoresValue}>[0.00, 1.00]</span>
            </div>
          </div>

          {/* Right — full swarm */}
          <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <span className={styles.panelTitle}>the swarm</span>
              <span className={styles.panelMeta}>profile-giga · 12 agents</span>
            </div>
            <div className={styles.voteTable}>
              <div className={styles.voteTableHeader}>
                <span className={styles.voteTableCol}>agent</span>
                <span className={styles.voteTableColBar}>vote distribution</span>
                <span className={styles.voteTableCol}>weight</span>
              </div>
              {EXAMPLE_VOTES.map((v, i) => {
                const relWeight = v.weight;
                return (
                  <div
                    key={i}
                    className={`${styles.agentRow} ${i === 0 ? styles.agentHighlight : ""}`}
                    style={relWeight >= 0.8 ? { background: `rgba(217, 119, 6, ${relWeight * 0.04})` } : undefined}
                  >
                    <span className={styles.agentModel}>{v.model}</span>
                    <div className={styles.agentVoteBar}>
                      <div className={styles.voteSegmentA} style={{ flex: Math.max(v.vote[0], 0.02) }} />
                      <div className={styles.voteSegmentB} style={{ flex: Math.max(v.vote[1], 0.02) }} />
                    </div>
                    <div className={styles.weightFader}>
                      <div className={styles.weightFaderTrack}>
                        <div
                          className={styles.weightFaderFill}
                          style={{
                            height: `${relWeight * 100}%`,
                            background: `rgba(217, 119, 6, ${0.3 + relWeight * 0.5})`,
                          }}
                        />
                      </div>
                      <span className={styles.agentWeight}>{v.weight.toFixed(1)}</span>
                    </div>
                  </div>
                );
              })}
            </div>
            <div className={styles.scoresRow}>
              <span className={styles.scoresLabel}>scores</span>
              <div className={styles.scoreBar}>
                <div className={styles.scoreSegmentA} style={{ flex: 0.62 }} />
                <div className={styles.scoreSegmentB} style={{ flex: 0.38 }} />
              </div>
              <span className={styles.scoresValue}>[0.62, 0.38]</span>
            </div>
          </div>
        </div>
      </section>

      {/* ── Agent-native ── */}
      {/* CONTENT: The invention + CLI story.
          Type: Differentiator — what makes ObjectiveAI different from every other AI tool.
          Key points to cover:
          - Functions are invented by agents via CLI, not written in a web UI
          - Recursive: agents spawn sub-agents for sub-functions
          - The viewer (desktop app) shows humans what agents are doing
          - "Functional for the agent, pretty for the human"
          Tone: Explain like infrastructure docs, not a pitch. */}
      <section className={styles.textSection}>
        <h2 className={styles.textSectionTitle}>agent-native</h2>
        <p className={styles.textSectionBody}>
          Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nemo enim ipsam
          voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia
          consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt.
        </p>
        <div className={styles.textColumns}>
          <div className={styles.textColumn}>
            <h3 className={styles.textColumnTitle}>lorem ipsum</h3>
            {/* CONTENT: CLI workflow — objectiveai invent, objectiveai execute, objectiveai train */}
            <p className={styles.textColumnBody}>
              Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet.
            </p>
          </div>
          <div className={styles.textColumn}>
            <h3 className={styles.textColumnTitle}>lorem ipsum</h3>
            {/* CONTENT: Viewer — real-time visualization, agent watching, execution streaming */}
            <p className={styles.textColumnBody}>
              Ut enim ad minima veniam, quis nostrum exercitationem ullam.
            </p>
          </div>
        </div>
      </section>

      {/* ── Concepts ── */}
      {/* CONTENT: Core primitives glossary.
          Type: Quick reference — not marketing, not docs. 2 sentences each max.
          These should feel like reading a man page header, not a product feature list. */}
      <section className={styles.textSection}>
        <h2 className={styles.textSectionTitle}>concepts</h2>
        <div className={styles.conceptGrid}>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>functions</h3>
            {/* CONTENT: Composable scoring pipelines. Data in, scores out. Tasks = vector completions or sub-functions. */}
            <p className={styles.conceptBody}>Lorem ipsum dolor sit amet.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>swarms</h3>
            {/* CONTENT: Collections of agents that vote together. Immutable. Weights are external. */}
            <p className={styles.conceptBody}>Consectetur adipiscing elit.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>profiles</h3>
            {/* CONTENT: Learned weights over agents. Trained from data, not hand-tuned. */}
            <p className={styles.conceptBody}>Sed do eiusmod tempor.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>agents</h3>
            {/* CONTENT: Fully-specified model configs. Content-addressed IDs via XXHash3-128. */}
            <p className={styles.conceptBody}>Incididunt ut labore et dolore.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>vector completions</h3>
            {/* CONTENT: The core primitive. Scores not text. Agents vote, weights combine, scores converge. */}
            <p className={styles.conceptBody}>Magna aliqua ut enim.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>invention</h3>
            {/* CONTENT: Agents create functions recursively via CLI. Agent-native, not human-authored. */}
            <p className={styles.conceptBody}>Ad minim veniam quis nostrud.</p>
          </div>
        </div>
      </section>

      {/* ── Directory (live from API) ── */}
      {/* CONTENT: Browse what exists. Populated from API when available, empty states when not.
          This section only shows content when the API is reachable. */}
      <section className={styles.directory}>
        <div className={styles.directorySection}>
          <div className={styles.sectionHeader}>
            <h2 className={styles.sectionTitle}>functions</h2>
            <Link href="/functions" className={styles.sectionLink}>browse all</Link>
          </div>
          {!loaded.fn ? <SectionLoader /> : sortedFunctions.length === 0 ? (
            <div className={styles.sectionEmpty}>functions are registered via the api and loaded from github</div>
          ) : (
            <div className={styles.directoryList}>
              {sortedFunctions.slice(0, 6).map((fn) => (
                <Link
                  key={`${fn.owner}/${fn.repository}`}
                  href={`/functions/${fn.owner}/${fn.repository}`}
                  className={styles.directoryRow}
                >
                  <span className={styles.directoryName}>{fn.name}</span>
                  <span className={styles.directoryMeta}>{fn.type}</span>
                  <span className={styles.directoryMeta}>{fn.taskCount} tasks</span>
                  {fn.subFunctions.length > 0 && (
                    <span className={styles.directoryMeta}>{fn.subFunctions.length} sub-functions</span>
                  )}
                </Link>
              ))}
            </div>
          )}
        </div>

        <div className={styles.directorySection}>
          <div className={styles.sectionHeader}>
            <h2 className={styles.sectionTitle}>swarms</h2>
            <Link href="/swarms" className={styles.sectionLink}>browse all</Link>
          </div>
          {!loaded.sw ? <SectionLoader /> : sortedSwarms.length === 0 ? (
            <div className={styles.sectionEmpty}>swarms are created at execution time</div>
          ) : (
            <div className={styles.directoryList}>
              {sortedSwarms.slice(0, 4).map((s) => (
                <Link key={s.id} href={`/swarms/${s.id}`} className={styles.directoryRow}>
                  <span className={styles.directoryName}>{swarmSummary(s)}</span>
                  <span className={styles.directoryMeta}>{s.totalAgentCount} agents</span>
                </Link>
              ))}
            </div>
          )}
        </div>

        <div className={styles.directorySection}>
          <div className={styles.sectionHeader}>
            <h2 className={styles.sectionTitle}>profiles</h2>
            <Link href="/profiles" className={styles.sectionLink}>browse all</Link>
          </div>
          {!loaded.pr ? <SectionLoader /> : profiles.length === 0 ? (
            <div className={styles.sectionEmpty}>profiles are loaded from github</div>
          ) : (
            <div className={styles.directoryList}>
              {profiles.map((p) => (
                <div key={p.name} className={styles.directoryRow}>
                  <span className={styles.directoryName}>{p.name}</span>
                  <span className={styles.directoryMeta}>{p.totalAgents} agents</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </section>

      {/* ── Footer ── */}
      <footer className={styles.landingFooter}>
        <a href="https://github.com/ObjectiveAI" target="_blank" rel="noopener noreferrer" className={styles.footerLink}>github</a>
        <a href="https://discord.gg/gbNFHensby" target="_blank" rel="noopener noreferrer" className={styles.footerLink}>discord</a>
        <a href="https://x.com/mkgores" target="_blank" rel="noopener noreferrer" className={styles.footerLink}>x.com/mkgores</a>
        <a href="https://x.com/ronald_obj_ai" target="_blank" rel="noopener noreferrer" className={styles.footerLink}>x.com/ronald_obj_ai</a>
      </footer>
    </div>
  );
}

/* ── Helpers ── */

function swarmSummary(s: SwarmMeta): string {
  const unique = [...new Set(s.agents.map((a) => {
    const short = a.model.includes("/") ? a.model.split("/").pop()! : a.model;
    return short.replace(/-chat(?=-)/i, "");
  }))];
  if (unique.length <= 2) return unique.join(" + ");
  return `${unique[0]} + ${unique[1]} +${unique.length - 2}`;
}

function SectionLoader() {
  return (
    <div className={styles.sectionLoading}>
      <span className={styles.loadingDot} />
    </div>
  );
}

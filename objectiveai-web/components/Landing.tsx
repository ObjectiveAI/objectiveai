"use client";

import { useState, type FormEvent } from "react";
import styles from "./Landing.module.css";

/* ── Static example — realistic votes from a startup-idea-ranker execution ── */

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
  const [email, setEmail] = useState("");
  const [emailState, setEmailState] = useState<"idle" | "submitting" | "success" | "error">("idle");
  const [errorMsg, setErrorMsg] = useState("something went wrong");

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
        <h1 className={styles.heroTitle}>the agentic collective judgment harness</h1>
        <p className={styles.heroBody}>
          Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod
          tempor incididunt ut labore et dolore magna aliqua.
        </p>

        {/* CTA — Ronald's hyperprompt / CLI install */}
        <div className={styles.install}>
          <span className={styles.installCmd}>$ lorem ipsum install objectiveai --lorem</span>
        </div>
      </section>

      {/* ── Proof — one agent vs the swarm ── */}
      <section className={styles.proof}>
        <p className={styles.proofLead}>your agent doesn&apos;t have to decide alone</p>

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

      {/* ── Structure — how functions compose ── */}
      <section className={styles.textSection}>
        <h2 className={styles.textSectionTitle}>structure</h2>
        <p className={styles.textSectionBody}>
          Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nemo enim ipsam
          voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia
          consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt.
        </p>
        <div className={styles.conceptGrid}>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>functions</h3>
            <p className={styles.conceptBody}>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>vector completions</h3>
            <p className={styles.conceptBody}>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>swarms</h3>
            <p className={styles.conceptBody}>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>profiles</h3>
            <p className={styles.conceptBody}>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>agents</h3>
            <p className={styles.conceptBody}>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>weights</h3>
            <p className={styles.conceptBody}>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
          </div>
        </div>
      </section>

      {/* ── Invention — agents build functions ── */}
      <section className={styles.textSection}>
        <h2 className={styles.textSectionTitle}>invention</h2>
        <p className={styles.textSectionBody}>
          Lorem ipsum dolor sit amet, consectetur adipiscing elit. Ut enim ad minima
          veniam, quis nostrum exercitationem ullam corporis suscipit laboriosam.
        </p>
        <div className={styles.textColumns}>
          <div className={styles.textColumn}>
            <h3 className={styles.textColumnTitle}>lorem ipsum</h3>
            <p className={styles.textColumnBody}>
              Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet.
            </p>
          </div>
          <div className={styles.textColumn}>
            <h3 className={styles.textColumnTitle}>lorem ipsum</h3>
            <p className={styles.textColumnBody}>
              Ut enim ad minima veniam, quis nostrum exercitationem ullam.
            </p>
          </div>
        </div>
      </section>

      {/* ── Footer ── */}
      <footer className={styles.landingFooter}>
        <div className={styles.footerLinks}>
          <a href="https://github.com/ObjectiveAI" target="_blank" rel="noopener noreferrer" className={styles.footerLink}>github</a>
          <a href="https://discord.gg/gbNFHensby" target="_blank" rel="noopener noreferrer" className={styles.footerLink}>discord</a>
          <a href="https://x.com/mkgores" target="_blank" rel="noopener noreferrer" className={styles.footerLink}>x.com/mkgores</a>
          <a href="https://x.com/ronald_obj_ai" target="_blank" rel="noopener noreferrer" className={styles.footerLink}>x.com/ronald_obj_ai</a>
        </div>
        <form className={styles.footerForm} onSubmit={handleSubmit} noValidate>
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
        {emailState === "error" && <p className={styles.footerError}>{errorMsg}</p>}
      </footer>
    </div>
  );
}

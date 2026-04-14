"use client";

import { EmailSignup } from "./EmailSignup";
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
  return (
    <div className={styles.landing}>

      {/* ── Hero ── */}
      <section className={styles.hero}>
        <h1 className={styles.heroTitle}>the agentic collective judgment harness</h1>
        <p className={styles.heroBody}>
          Any agent can call out to a swarm of models for collective judgment; routing decisions through recursive scoring trees that produce a vector of scores across every option. No fine-tuning. ObjectiveAI learns weights.
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
          agent.json ← swarm.json ← profile.json ← function.json. Core primitives that reference each other. A function references a profile. A profile references a swarm with weights. A swarm references agents. Each link is either inline or a remote path.
        </p>
        <div className={styles.conceptGrid}>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>functions</h3>
            <p className={styles.conceptBody}>Composable scoring pipelines. Data in, scores out. Recursive decision trees that contain vector completions, nested function calls, and map operations, arbitrarily composed.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>vector completions</h3>
            <p className={styles.conceptBody}>The core primitive. Give a swarm a prompt and a set of possible responses. Each agent votes. Votes combine with weights to produce a score vector that sums to 1.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>swarms</h3>
            <p className={styles.conceptBody}>A named collection of agents used together for collective judgment. Does not contain weights, as weights are external, learnable, and never baked in.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>profiles</h3>
            <p className={styles.conceptBody}>Learned weight configurations across agents and tasks. Human-readable. Looking at the weights tells you about the perspective being simulated.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>agents</h3>
            <p className={styles.conceptBody}>A complete configuration of a single upstream model: sampling parameters, prompt framing, tool access, voting behavior. Content-addressed; same configuration, same ID, always.</p>
          </div>
          <div className={styles.concept}>
            <h3 className={styles.conceptName}>weights</h3>
            <p className={styles.conceptBody}>Parameters controlling each agent's influence over the final score. Supplied separately from the swarm, learned through profile training. The only thing that changes between one perspective and another.</p>
          </div>
        </div>
      </section>

      {/* ── Invention — agents build functions ── */}
      <section className={styles.textSection}>
        <h2 className={styles.textSectionTitle}>invention</h2>
        <p className={styles.textSectionBody}>
          Agents don&apos;t just execute functions. They build them. Pick an agent, give them a spec. They invent the function, spawn more agents to invent the sub-functions, and those spawn more for theirs. The result is a complete decision tree, ready to deploy and train.
        </p>
        <div className={styles.textColumns}>
          <div className={styles.textColumn}>
            <h3 className={styles.textColumnTitle}>invent</h3>
            <p className={styles.textColumnBody}>
              Give an agent a description of what you want to score. They reason about what qualities matter, design the input schema, write the task tree, and validate at each step. Recursive; sub-functions are invented concurrently by their own agents.
            </p>
          </div>
          <div className={styles.textColumn}>
            <h3 className={styles.textColumnTitle}>deploy</h3>
            <p className={styles.textColumnBody}>
              The result is a complete function.json. Train a profile on your data to learn the right weights. Deploy them, call them from your agent, or let your agent do all of that on their own.
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
        <EmailSignup
          formClassName={styles.footerForm}
          inputClassName={styles.emailInput}
          buttonClassName={styles.submitBtn}
          confirmationClassName={styles.confirmation}
          errorClassName={styles.footerError}
        />
      </footer>
    </div>
  );
}

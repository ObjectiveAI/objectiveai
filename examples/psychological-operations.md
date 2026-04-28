# psychological-operations

**Repository:** <https://github.com/WiggidyW/psychological-operations>

## What it is

`psychological-operations` is an agentic X (Twitter) scraper and scoring
pipeline that uses ObjectiveAI as its evaluation substrate. It pairs a
human-driven Chrome session (typed search via real keyboard automation,
intentionally not headless) with ObjectiveAI functions, swarms, and
profiles to rank scraped tweets along arbitrary operator-defined axes.

The CLI is organized around three primary objects:

- **Scrapes** — declarative search-job definitions (per-handle search
  filters, target tweet counts, min-engagement gates, scrape-time
  validation thresholds, output tags). `scrapes run` opens one Chrome
  session at `x.com`, then for each filter types the query into the
  in-page search bar, clicks the *Latest* tab, and scrolls/parses
  results into a SQLite store, tagged with the scrape's tags.

- **PsyOps** — declarative scoring jobs that pull tagged posts out of
  the SQLite store and run them through an ObjectiveAI function under
  a chosen swarm/profile and execution strategy (e.g. Swiss-System
  tournament). `psyops run` schedules eligible psyops in rounds; later
  rounds pick up cascading psyops whose `Source.min_score` depends on
  scores written by earlier rounds.

- **Inventions** — wrapper around `objectiveai functions inventions
  recursive create alpha-{scalar,vector}` that attaches the project's
  canonical post input schema (`text` + `images` + `videos`) to the
  invention so the Claude/OpenAI agent inventing the function knows
  exactly what fields it has to work with.

## How ObjectiveAI fits in

Every psyop binds:

- a **function** (typically published to GitHub or a local filesystem
  store, content-addressed, often a recursive
  `alpha.{scalar,vector}.branch.function` whose leaves are
  `vector.completion` tasks),
- a **profile** (an Auto profile = inline swarm; or a remote profile
  reference),
- and a **strategy** (`default` for scalar/vector, or `swiss_system`
  with `pool` + `rounds` for tournament-style ranking of vector
  responses).

The scoring swarm is generally a single OpenRouter agent configured
with `output_mode: json_schema` and `top_logprobs: 20`, which lets the
project recover the model's full preference distribution over candidate
responses from a single API call (instead of just the argmax pick).

## Pilot study

A small pilot study built with the project ranked recent tweets from
the personal accounts of 33 Y Combinator W22 CEOs along an
*unsettlingness* axis, decomposed into three sub-judgments (visceral
wrongness, tonal dissonance, lingering disquiet) that were themselves
invented by a Claude Opus agent under a controlled spec. The
sub-functions and the per-company-top-3 meta-pool ranking that combines
them are content-addressed and reproducible. Results, methodology, and
the published function/profile/swarm artifacts are documented at:

<https://github.com/WiggidyW/psychological-operations-unsettlingness>

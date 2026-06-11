#!/usr/bin/env bash
# Publishes the entire ObjectiveAI release across all registries, in
# dependency order. Each wave dispatches its per-package publish scripts
# in parallel; the next wave only starts after every registry in the
# prior wave reports the new version live. No manual retries.
#
# Idempotent: re-running after a partial failure skips packages that are
# already live at the current VERSION and resumes from the rest.
#
# Usage:
#   bash publish.sh                # full production release (dispatches GHA)
#   bash publish.sh --build-only   # local sanity check across all packages
#
# Requires: gh CLI authenticated; relevant secrets set on the repo
# (CARGO_REGISTRY_TOKEN, NPM_TOKEN, PYPI_API_TOKEN).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
VERSION="$(awk '/^version = "/ { gsub(/version = "|"/, ""); print; exit }' "$REPO_ROOT/objectiveai-sdk-rs/Cargo.toml")"
TIMEOUT_SECS=1200   # 20 minutes per package
POLL_SECS=15

# Per-package version: the Go SDK versions independently via
# objectiveai-sdk-go/version.txt (its publish.sh reads the same file);
# every other package follows the canonical lockstep VERSION above.
version_for_dir() {
  if [[ "$1" == "objectiveai-sdk-go" ]]; then
    tr -d ' \r\n' < "$REPO_ROOT/objectiveai-sdk-go/version.txt"
  else
    printf '%s' "$VERSION"
  fi
}

# Entries: "<dir>|<registry>|<published-name>"
# registry ∈ {crates, pypi, npm, go, github-release}
WAVE_1=(
  "objectiveai-sdk-rs-macros|crates|objectiveai-sdk-macros"
  "objectiveai-sdk-go|go|objectiveai-sdk-go"
)
WAVE_2=(
  "objectiveai-sdk-rs|crates|objectiveai-sdk"
)
WAVE_3=(
  "objectiveai-mcp-proxy|crates|objectiveai-mcp-proxy"
  "objectiveai-mcp-filesystem|crates|objectiveai-mcp-filesystem"
  "objectiveai-mcp|crates|objectiveai-mcp"
  "objectiveai-sdk-py|pypi|objectiveai-sdk"
  "objectiveai-sdk-js|npm|@objectiveai/sdk"
)
WAVE_4=(
  "objectiveai-api|crates|objectiveai-api"
  "objectiveai-cli|crates|objectiveai-cli"
  "objectiveai-cocoindex|pypi|objectiveai-cocoindex"
)

# ── --build-only fast path: everything in parallel, no wave/wait logic ──
if [[ "${1:-}" == "--build-only" ]]; then
  pids=()
  for entry in "${WAVE_1[@]}" "${WAVE_2[@]}" "${WAVE_3[@]}" "${WAVE_4[@]}"; do
    dir="${entry%%|*}"
    bash "$REPO_ROOT/$dir/publish.sh" --build-only &
    pids+=($!)
  done
  failed=false
  for pid in "${pids[@]}"; do wait "$pid" || failed=true; done
  $failed && exit 1
  exit 0
fi

# ── registry liveness probe ─────────────────────────────────────────────
is_live() {
  local registry="$1" name="$2" version="$3"
  case "$registry" in
    crates)
      curl -fsS -o /dev/null 2>/dev/null "https://crates.io/api/v1/crates/$name/$version"
      ;;
    pypi)
      curl -fsS -o /dev/null 2>/dev/null "https://pypi.org/pypi/$name/$version/json"
      ;;
    npm)
      curl -fsS -o /dev/null 2>/dev/null "https://registry.npmjs.org/$name/$version"
      ;;
    go)
      git -C "$REPO_ROOT" ls-remote --tags origin "refs/tags/$name/v$version" \
        | grep -q "refs/tags/$name/v$version$"
      ;;
    github-release)
      gh release view "v$version" \
        --repo "$(gh repo view --json nameWithOwner -q .nameWithOwner)" \
        >/dev/null 2>&1
      ;;
    *)
      echo "unknown registry: $registry" >&2; return 2
      ;;
  esac
}

wait_for_live() {
  local label="$1" registry="$2" name="$3" version="$4"
  local started=$SECONDS
  printf "  · waiting for %s on %s..." "$label" "$registry"
  while (( SECONDS - started < TIMEOUT_SECS )); do
    if is_live "$registry" "$name" "$version"; then
      printf " live (%ds)\n" $(( SECONDS - started ))
      return 0
    fi
    sleep "$POLL_SECS"
    printf "."
  done
  printf " TIMED OUT (%ds)\n" $(( SECONDS - started ))
  return 1
}

# ── wave executor ───────────────────────────────────────────────────────
run_wave() {
  local wave_name="$1"; shift
  local entries=("$@")

  echo
  echo "=== $wave_name ==="

  # 1. Dispatch each per-package script in parallel — but skip any
  #    package already live at the current VERSION (idempotence).
  local pids=() labels=() to_wait=()
  for entry in "${entries[@]}"; do
    local dir registry name pkg_version
    IFS='|' read -r dir registry name <<<"$entry"
    [[ -z "$name" ]] && name="$dir"
    pkg_version="$(version_for_dir "$dir")"
    if is_live "$registry" "$name" "$pkg_version"; then
      echo "  · $dir already live at $pkg_version on $registry — skip"
      continue
    fi
    bash "$REPO_ROOT/$dir/publish.sh" &
    pids+=($!)
    labels+=("$dir")
    to_wait+=("$entry")
  done

  # 2. Block on dispatch completion (a few seconds each — just the
  #    `gh workflow run` call returning OK, or the sdk-go tag push).
  local dispatch_failed=false
  for i in "${!pids[@]}"; do
    if ! wait "${pids[$i]}"; then
      echo "  ✗ dispatch failed: ${labels[$i]}" >&2
      dispatch_failed=true
    fi
  done
  $dispatch_failed && return 1

  # 3. Poll each registry until the new version is live.
  for entry in "${to_wait[@]}"; do
    local dir registry name pkg_version
    IFS='|' read -r dir registry name <<<"$entry"
    [[ -z "$name" ]] && name="$dir"
    pkg_version="$(version_for_dir "$dir")"
    wait_for_live "$dir" "$registry" "$name" "$pkg_version" || return 1
  done
}

# ── go ──────────────────────────────────────────────────────────────────
echo "Publishing ObjectiveAI $VERSION across all registries..."
run_wave "Wave 1 — leaves (no upstream deps)"               "${WAVE_1[@]}"
run_wave "Wave 2 — depends on objectiveai-sdk-macros"       "${WAVE_2[@]}"
run_wave "Wave 3 — depends on objectiveai-sdk"              "${WAVE_3[@]}"
run_wave "Wave 4 — depend on wave-3 crates: api (mcp-proxy), cli (mcp), cocoindex (sdk-py)" "${WAVE_4[@]}"
echo
echo "✓ All packages published at $VERSION"

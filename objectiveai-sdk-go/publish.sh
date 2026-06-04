#!/usr/bin/env bash
# Publishes objectiveai-sdk-go by pushing a git tag.
#
# Go modules are not registry-hosted — `go get` resolves versions from
# git tags. To publish a new version, tag the current HEAD as
# `objectiveai-sdk-go/v<X.Y.Z>` and push.
#
# This script is local-only (no GHA workflow) — pushing a tag from a
# server-side checkout would require the same git auth that the local
# operator already has, with no extra benefit.
#
# Usage:
#   bash objectiveai-sdk-go/publish.sh                # tag + push current HEAD
#   bash objectiveai-sdk-go/publish.sh --build-only   # go build + go test (no tag)
#
# `--test` is not supported — Go has no test registry.
#
# Output is captured to .logs/publish/objectiveai-sdk-go.txt.
#
# Pre-flight (the script enforces these):
#   - working tree is clean
#   - on branch `main`
#   - tag `objectiveai-sdk-go/v<version>` does not already exist on origin
#
# Version is read from `objectiveai-sdk-go/version.txt` — the Go SDK's
# OWN version, independent of the repo's lockstep version. `version.sh`
# writes this file on a full lockstep bump, but it can be edited alone
# to release (or hold back) the Go SDK independently.

set -euo pipefail

MODULE="objectiveai-sdk-go"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/publish"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

BUILD_ONLY=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --test)         echo "ERROR: Go has no test registry; --test is not supported." >&2; exit 1 ;;
    --build-only)   BUILD_ONLY=true; shift ;;
    *)              echo "Unknown flag: $1" >&2; exit 1 ;;
  esac
done

# Read the Go SDK's own version from version.txt
VERSION="$(tr -d ' \r\n' < "$SCRIPT_DIR/version.txt" 2>/dev/null || true)"
if [[ -z "$VERSION" ]]; then
  echo "ERROR: could not read version from $SCRIPT_DIR/version.txt" >&2
  exit 1
fi
TAG="objectiveai-sdk-go/v$VERSION"

if $BUILD_ONLY; then
  run_local() {
    echo "Version: $VERSION (would tag as $TAG)"
    echo "Running go build ./... + go test ./... -count=1..."
    ( cd "$SCRIPT_DIR" && go build ./... ) || return $?
    ( cd "$SCRIPT_DIR" && go test ./... -count=1 ) || return $?
    echo "--build-only specified; skipping tag + push."
  }

  if run_local > "$LOG_FILE" 2>&1; then
    echo "$MODULE: BUILT (local)"
  else
    echo "$MODULE: ERROR (see $LOG_FILE)"
    exit 1
  fi
  exit 0
fi

run_remote() {
  # ── pre-flight ───────────────────────────────────────────────────────────────
  if [[ -n "$(git -C "$REPO_ROOT" status --porcelain)" ]]; then
    echo "ERROR: working tree is not clean. Commit or stash before publishing." >&2
    return 1
  fi

  CURRENT_BRANCH="$(git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD)"
  if [[ "$CURRENT_BRANCH" != "main" ]]; then
    echo "ERROR: not on branch main (currently on $CURRENT_BRANCH). Refusing to tag." >&2
    return 1
  fi

  # Refuse if tag already exists on origin
  if git -C "$REPO_ROOT" ls-remote --tags origin "refs/tags/$TAG" | grep -q "$TAG"; then
    echo "ERROR: tag $TAG already exists on origin. Bump the version first." >&2
    return 1
  fi

  echo "Tagging HEAD as $TAG..."
  git -C "$REPO_ROOT" tag -a "$TAG" -m "objectiveai-sdk-go v$VERSION"
  echo "Pushing tag to origin..."
  git -C "$REPO_ROOT" push origin "$TAG"
  echo
  echo "Tag pushed. Consumers can now run:"
  echo "  go get github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go@v$VERSION"
}

if run_remote 2>&1 | tee "$LOG_FILE"; then
  echo "$MODULE: TAGGED $TAG"
else
  echo "$MODULE: ERROR (see $LOG_FILE)"
  exit 1
fi

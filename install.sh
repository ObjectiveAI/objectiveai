#!/usr/bin/env bash
# Build and install every Diverge program into a config directory's
# `bin/`: the CLI, the daemon, the provider, the Postgres supervisor
# and the container proxy, each by its own crate's `install.sh`.
#
#   ./install.sh <config-dir> (--debug | --release)
#
# The arguments are the per-crate scripts' and are handed to each
# unchanged: the config directory the binaries land under, and exactly
# one profile. The five run ONE AFTER ANOTHER, never at once: every
# cargo invocation in this workspace shares one `target/` and its
# build-directory lock, so parallel runs would only block on each
# other — and a build that fails stops the sequence there, with the
# earlier binaries installed and the later ones untouched. The order
# is the one the programs look one another up in: the CLI last, so
# nothing it starts is older than itself.
set -euo pipefail

usage() {
    echo "usage: $0 <config-dir> (--debug | --release)" >&2
    exit 2
}

[[ $# -eq 2 ]] || usage
config=""
profile=""
for arg in "$@"; do
    case "$arg" in
        --debug|--release)
            [[ -z "$profile" ]] || usage
            profile="$arg"
            ;;
        --*) usage ;;
        *)
            [[ -z "$config" ]] || usage
            config="$arg"
            ;;
    esac
done
[[ -n "$config" && -n "$profile" ]] || usage

workspace="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
for crate in diverge-postgres diverge-container-proxy diverge-daemon diverge-provider diverge-cli; do
    echo "== $crate"
    "$workspace/$crate/install.sh" "$config" "$profile"
done
echo "installed every Diverge program under $config/bin ($profile)"

#!/usr/bin/env bash
# Build diverge-issuer and install its binary into a config directory's `bin/`.
#
#   diverge-issuer/install.sh <config-dir> (--debug | --release)
#
# <config-dir> is the Diverge config directory — the root the programs
# are started against (`--config <dir>`, or `~/.diverge`) — and the
# binary lands at <config-dir>/bin/diverge-issuer{.exe}, beside the other Diverge
# binaries installed the same way, which is where they look for one
# another. Exactly one profile is given; both or neither is an error.
#
# Multiplatform: runs under bash on Linux, macOS and Windows (Git Bash
# or MSYS); the host's OS and architecture are read, never chosen.
set -euo pipefail

usage() {
    echo "usage: $0 <config-dir> (--debug | --release)" >&2
    exit 2
}

config=""
profile=""
for arg in "$@"; do
    case "$arg" in
        --debug|--release)
            if [[ -n "$profile" ]]; then
                echo "error: --debug and --release are each given at most once, and not together" >&2
                usage
            fi
            profile="${arg#--}"
            ;;
        --*)
            echo "error: unknown option $arg" >&2
            usage
            ;;
        *)
            if [[ -n "$config" ]]; then
                echo "error: one config directory, given once" >&2
                usage
            fi
            config="$arg"
            ;;
    esac
done
[[ -n "$config" && -n "$profile" ]] || usage

workspace="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) host_exe=".exe" ;;
    *) host_exe="" ;;
esac
case "$(uname -m)" in
    x86_64|amd64) arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *) echo "error: unsupported architecture $(uname -m)" >&2; exit 1 ;;
esac

# The host's own target: the binary runs where it is built.
installed="diverge-issuer$host_exe"
flags=()
[[ "$profile" == "release" ]] && flags+=(--release)
(cd "$workspace" && cargo build -p diverge-issuer ${flags[@]+"${flags[@]}"})
built="$workspace/target/$profile/$installed"

[[ -f "$built" ]] || { echo "error: $built was not built" >&2; exit 1; }
mkdir -p "$config/bin"
# Staged beside its place and moved in, so a reader never sees half a
# binary; a running copy keeps its old file open on Unix and refuses
# the move on Windows, where the program is stopped first.
cp "$built" "$config/bin/$installed.installing"
mv -f "$config/bin/$installed.installing" "$config/bin/$installed"
chmod +x "$config/bin/$installed"
echo "installed $config/bin/$installed ($profile)"

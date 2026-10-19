#!/bin/sh
# Run this feature's own build; GitHub plugin checkouts share one build cache and never load an upstream release.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PATH="${PATH:-}:/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin:$HOME/.local/bin"; export PATH
local_program="$root/target/quick/herdr-projects"
run() { if [ "${1:-}" = --fetch ]; then exit 0; fi; exec "$program" "$@"; }
if [ -x "$local_program" ] && [ -z "$(find "$root/src" "$root/crates" "$root/Cargo.toml" "$root/Cargo.lock" -newer "$local_program" 2>/dev/null | head -n 1)" ]; then
  program="$local_program"; run "$@"
fi
cache="${XDG_CACHE_HOME:-$HOME/.cache}/herdr-control-build"
mkdir -p "$cache"
program="$cache/target/quick/herdr-projects"
fingerprint=$(cd "$root" && { find src crates -type f -exec shasum -a 256 {} +; shasum -a 256 Cargo.toml Cargo.lock; } | LC_ALL=C sort | shasum -a 256 | cut -d' ' -f1)
fresh() { [ -x "$program" ] && [ "$(cat "$cache/source.sha256" 2>/dev/null || true)" = "$fingerprint" ]; }
fresh && run "$@"
lock="$cache/build.lock"
tries=0
until mkdir "$lock" 2>/dev/null; do
  owner=$(cat "$lock/pid" 2>/dev/null || true)
  if [ -n "$owner" ] && ! kill -0 "$owner" 2>/dev/null; then rm -rf "$lock"; continue; fi
  tries=$((tries + 1))
  [ "$tries" -lt 600 ] || { echo 'Mission-Control-Build ist noch gesperrt. Bitte erneut öffnen.' >&2; exit 1; }
  sleep 1
  fresh && run "$@"
done
printf '%s\n' "$$" > "$lock/pid"
trap 'rm -rf "$lock"' EXIT
trap 'exit 1' INT TERM HUP
if ! fresh; then
  command -v cargo >/dev/null 2>&1 || { echo 'Herdr Mission Control benötigt Rust für diesen Feature-Branch.' >&2; exit 1; }
  (cd "$root"; . scripts/build-cache.sh; . scripts/share-cores.sh; CARGO_TARGET_DIR="$cache/target" cargo build --locked --profile quick --quiet)
  printf '%s\n' "$fingerprint" > "$cache/source.sha256"
fi
rm -rf "$lock"
trap - EXIT INT TERM HUP
run "$@"

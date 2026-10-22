#!/bin/sh
# The one command that runs the tests, for everyone: `npm run check` (`npm test`, `sh scripts/check.sh`). It builds the
# program, writes the made tables when they are out of date (and says so: commit them), checks the repo's own rules and
# runs every test. `--all` runs everything (as a run with no names does); names run one area's test files or the
# tests whose names hold them (`npm run check board-panel textbox`); `--since <commit>` is the shorter check of a fix
# round: only the test files the change since that commit reaches (scripts/changed-tests.sh), or all of them where that
# is not known; `--fresh` runs again what already passed. What
# passed is remembered for the exact code it passed on (every file of the folder, committed or not, and the compiler): asked again, it answers at
# once. How it works: docs/testing.md.
set -e
# A check that stops says why, also when a step stopped it with no word of its own (`set -e`).
step="starting"
trap 'st=$?; sc_leave 2>/dev/null; [ "$st" = 0 ] || echo "The check stopped while $step (exit $st)." >&2' EXIT
quit() { echo "$1" >&2; trap - EXIT; exit 2; }
here=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$here"
# The folder checked is this one. The board runs a task's check with HERDR_PROJECTS_REPO set to your own checkout
# (finish/pipeline.rs), and the program's made pages and rules follow that variable (docs.rs): taken along, the check
# would read and write your checkout, not the task's.
unset HERDR_PROJECTS_REPO
fresh=0; names=""; since=""; again=0
while [ $# -gt 0 ]; do
  case "$1" in
    --all) ;;
    --fresh) fresh=1 ;;
    # The board gives it the commit the full check passed on; none given, it is not known, and everything runs.
    --since) again=1; case "${2:-}" in ""|--*) ;; *) since="$2"; shift ;; esac ;;
    --since=*) again=1; since="${1#--since=}" ;;
    --*) quit "check does not know $1. It takes: --all, --fresh, --since <commit>, and names of tests." ;;
    *) names="$names $1" ;;
  esac
  shift
done
[ "$again" = 0 ] || [ -z "$names" ] || quit "check takes names of tests or --since, not both."
say() { [ -n "$HERDR_PROJECTS_PROGRESS" ] && printf '%s\n' "$1" > "$HERDR_PROJECTS_PROGRESS" || true; }
if ! command -v cargo >/dev/null 2>&1; then
  for d in "$HOME/.cargo/bin"; do [ ! -x "$d/cargo" ] || PATH="$d:$PATH"; done
fi
command -v cargo >/dev/null 2>&1 || quit "The check needs Rust (cargo): https://rustup.rs"
# Compiles through the build cache every worktree shares, when the workspace setup installed it, on its share of the
# cores: counted among the checks running now from here until it ends (docs/testing.md).
. scripts/build-cache.sh
. scripts/share-cores.sh
# The code as it stands, new files and uncommitted changes too, and the compiler that builds it.
cache="${XDG_CACHE_HOME:-$HOME/.cache}/herdr-projects-check"
stamp=""
if git rev-parse --git-dir >/dev/null 2>&1; then
  idx=$(mktemp); cp "$(git rev-parse --git-path index)" "$idx" 2>/dev/null || true
  tree=$(GIT_INDEX_FILE="$idx" git add -A >/dev/null 2>&1 && GIT_INDEX_FILE="$idx" git write-tree 2>/dev/null || true); rm -f "$idx"
  [ -n "$tree" ] && stamp="$tree-$(rustc -V | tr -c 'A-Za-z0-9.\n' '_')-$(printf '%s' "$names" | tr -c 'A-Za-z0-9_\n' '_')"
fi
# The shorter check is remembered as itself, from its commit: what passed is its own pick of the tests. What the whole
# check passed answers it too.
whole="$stamp"
if [ "$again" = 1 ] && [ -n "$stamp" ]; then
  stamp="$stamp-since_$(printf '%s' "$since" | tr -c 'A-Za-z0-9_\n' '_')"
  if [ "$fresh" = 0 ] && [ -f "$cache/$whole" ]; then say '{"state":"known"}'; echo "The whole check already passed for this exact code."; exit 0; fi
fi
if [ "$fresh" = 0 ] && [ -n "$stamp" ] && [ -f "$cache/$stamp" ]; then
  say '{"state":"known"}'
  echo "The check already passed for this exact code."
  exit 0
fi
started=$(date +%s)
sc_join
say "0/3 steps"
step="building the program"
sc_cargo build --profile quick --quiet
say "1/3 steps"
# The made tables, written when out of date; then the rules (tests/guard.rs runs them too).
step="writing the made tables"
./target/quick/herdr-projects docs
step="checking the repo's rules"
./target/quick/herdr-projects docs --guard
say "2/3 steps"
# The tests, split over several processes at once (scripts/run-tests.sh): every test, or those of the names given.
step="running the tests"
if [ "$again" = 1 ]; then
  step="picking the tests the change reaches"
  picked=$(sh scripts/changed-tests.sh "$since")
  step="running the tests"
  if [ "$picked" = all ]; then
    names=""
  else
    names="--units $(printf '%s\n' "$picked" | sed 's/^/=/' | tr '\n' ' ')"
    echo "The shorter check: the test files the change since $since reaches ($(printf '%s\n' "$picked" | grep -c .) of $(ls tests/*.rs | grep -vc '^tests/main.rs$')), and the tests beside the code: $(printf '%s\n' "$picked" | tr '\n' ' ')"
  fi
fi
# shellcheck disable=SC2086
sh scripts/run-tests.sh $names
say "3/3 steps"
if [ -n "$stamp" ]; then { mkdir -p "$cache" && echo "passed in $(( $(date +%s) - started ))s" > "$cache/$stamp"; } || true; fi
# A shorter check that ran everything was the whole check.
if [ "$again" = 1 ] && [ -z "$names" ] && [ -n "$whole" ]; then { echo "passed in $(( $(date +%s) - started ))s" > "$cache/$whole"; } || true; fi
echo "The check passed in $(( $(date +%s) - started ))s."

#!/bin/sh
# How many cores one task's build and tests take: the machine's cores shared out between the checks and builds of this
# repo running on it now, so a lone check takes all of them, two take half each, and none fewer than one. Sourced by
# scripts/check.sh, scripts/setup.sh and scripts/run-tests.sh; docs/testing.md. Sourced under `set -e`: no step here may
# fail, so a machine without a cache folder (CI, a fresh clone) still gets its share and goes on.
#
# A running check or setup has a file in ~/.cache/herdr-projects/running (`sc_join`, removed by `sc_leave`): named by
# its process, holding when that process started, so one that died without removing it (killed) is not counted, and a
# number used again by another process is not taken for it. `sc_share` counts them anew before each build and test run:
# a check that starts while another runs takes half, and takes all again when the other is done.
#
# Cargo is given its share as `-j` (`sc_cargo`), never in CARGO_BUILD_JOBS: the build cache keys every compile on the
# CARGO_ variables cargo hands the compiler, so a job count that changes from run to run there would miss the cache
# every time. CARGO_BUILD_JOBS or RUST_TEST_THREADS set by you wins over the share.
sc_cores=$(getconf _NPROCESSORS_ONLN 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 1)
case "$sc_cores" in ''|0*|*[!0-9]*) sc_cores=1 ;; esac
sc_dir="${XDG_CACHE_HOME:-$HOME/.cache}/herdr-projects/running"
sc_mine=""
sc_started() { LC_ALL=C ps -o lstart= -p "$1" 2>/dev/null || true; }
# One already counted for the check that started this script (HERDR_PROJECTS_RUNNING, its file) is not counted again.
sc_join() {
  [ -z "$sc_mine" ] && { [ -z "$HERDR_PROJECTS_RUNNING" ] || [ ! -f "$HERDR_PROJECTS_RUNNING" ]; } || return 0
  { mkdir -p "$sc_dir" && sc_started $$ > "$sc_dir/.$$" && mv "$sc_dir/.$$" "$sc_dir/$$" && sc_mine="$sc_dir/$$"; } 2>/dev/null || true
  [ -z "$sc_mine" ] || { HERDR_PROJECTS_RUNNING=$sc_mine; export HERDR_PROJECTS_RUNNING; }
}
sc_leave() { [ -z "$sc_mine" ] || rm -f "$sc_mine" 2>/dev/null || true; sc_mine=""; }
# The share now: sc_jobs for cargo, sc_threads for the tests.
sc_share() {
  sc_n=0
  for sc_f in "$sc_dir"/*; do
    sc_p=${sc_f##*/}
    case "$sc_p" in ''|*[!0-9]*) continue ;; esac
    sc_was=$(cat "$sc_f" 2>/dev/null || true)
    if kill -0 "$sc_p" 2>/dev/null && [ "$(sc_started "$sc_p")" = "$sc_was" ]; then sc_n=$((sc_n + 1))
    else rm -f "$sc_f" 2>/dev/null || true; fi
  done
  [ "$sc_n" -ge 1 ] || sc_n=1
  sc_each=$(( (sc_cores + sc_n - 1) / sc_n ))
  sc_jobs="${CARGO_BUILD_JOBS:-$sc_each}"; sc_threads="${RUST_TEST_THREADS:-$sc_each}"
  case "$sc_jobs" in ''|0*|*[!0-9]*) sc_jobs=$sc_each ;; esac
  case "$sc_threads" in ''|0*|*[!0-9]*) sc_threads=$sc_each ;; esac
}
# `sc_cargo build …`: cargo on the share of now, with CARGO_BUILD_JOBS kept out of what the compiler sees.
sc_cargo() {
  sc_share
  sc_sub=$1; shift
  (unset CARGO_BUILD_JOBS; exec cargo "$sc_sub" -j "$sc_jobs" "$@")
}

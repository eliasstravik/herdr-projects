#!/bin/sh
# Runs the tests for the check (scripts/check.sh), split over several processes of the test programs at once: a test
# that makes a world sets the process's environment, so in one process those tests take turns, and the only way to run
# them side by side is more processes. `sh scripts/run-tests.sh [names]`: no name runs every test; a name that is an
# area (`board-panel`: tests/board_panel.rs and tests/board_panel_*.rs) runs those files, any other name the tests whose
# own names hold it. How many processes: HERDR_PROJECTS_TEST_PROCS, else twice its share of the cores, counted when the
# tests start among the checks running then (scripts/share-cores.sh), as a process that takes turns uses about half a
# core. For the shorter check of a fix round (scripts/check.sh --since): `=name` runs exactly that file's tests, and `--units` every test outside tests/ (those in
# src/ and the doc tests). docs/testing.md.
set -e
mods=""; words=""; units=""
for n in "$@"; do
  case "$n" in
    --units) units=1; continue ;;
    =*) [ -f "tests/${n#=}.rs" ] && mods="$mods|${n#=}"; continue ;;
  esac
  f=$(printf '%s' "$n" | tr - _); found=""
  for t in tests/"$f".rs tests/"$f"_*.rs; do
    [ -f "$t" ] && [ "$t" != tests/main.rs ] && { found=1; mods="$mods|$(basename "$t" .rs)"; }
  done
  [ -n "$found" ] || words="$words $n"
done
. scripts/share-cores.sh
# Built once, compile errors shown as cargo shows them; the test programs' paths come from cargo's own report (not the
# program itself, which cargo builds for the tests too).
sc_join
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"; sc_leave' EXIT
sc_cargo test --profile quick --quiet --no-run --message-format=json-render-diagnostics > "$tmp/built"
bins=$(grep '"profile":{[^}]*"test":true' "$tmp/built" | sed -n 's/.*"executable":"\([^"]*\)".*/\1/p')
[ -n "$bins" ] || { echo "cargo built no test program." >&2; exit 101; }
sc_share
procs="${HERDR_PROJECTS_TEST_PROCS:-$((sc_threads * 2))}"
case "$procs" in ''|0*|*[!0-9]*) procs=1 ;; esac
# The tests each program is to run, by their exact names: an area's files (in the one test program, tests/main.rs),
# the tests whose names hold a word, or all. `settings::` as a filter would also run `web_settings::`: areas are
# picked by their exact module.
i=0
for b in $bins; do
  : > "$tmp/all"
  # shellcheck disable=SC2086
  case "$(basename "$b")" in
    main-*) if [ -z "$units" ] && { [ -z "$mods" ] || [ -n "$words" ]; }; then "$b" --list --format terse $words >> "$tmp/all"; fi ;;
    *) if [ -n "$units" ] || [ -z "$mods" ] || [ -n "$words" ]; then "$b" --list --format terse $words >> "$tmp/all"; fi ;;
  esac
  case "$(basename "$b")" in main-*) if [ -n "$mods" ]; then "$b" --list --format terse > "$tmp/mods"; grep -E "^(${mods#|})::" "$tmp/mods" >> "$tmp/all" || true; fi ;; esac
  sed -n 's/: test$//p' "$tmp/all" | sort -u > "$tmp/list"
  # Dealt out in turn, so each process gets some of every file's tests (the files differ, the tests of one file less).
  k=0
  while [ "$k" -lt "$procs" ]; do
    awk -v n="$procs" -v k="$k" '(NR - 1) % n == k' "$tmp/list" > "$tmp/$i.names"
    if [ -s "$tmp/$i.names" ]; then echo "$b" > "$tmp/$i.bin"; i=$((i + 1)); fi
    k=$((k + 1))
  done
done
started=$(date +%s)
# The examples in the code's own comments (doc tests), compiled by cargo: beside the areas, which are tests/ files only.
doc=0
# shellcheck disable=SC2086
if [ -n "$units" ] || [ -z "$mods" ] || [ -n "$words" ]; then
  sc_cargo test --profile quick --quiet --doc -- $words > "$tmp/doc.out" 2>&1 || { cat "$tmp/doc.out"; exit 101; }
  doc=$(sed -n 's/^test result: [A-Za-z]*\. \([0-9]*\) passed.*/\1/p' "$tmp/doc.out" | tail -1)
fi
n=0
while [ "$n" -lt "$i" ]; do
  # shellcheck disable=SC2046
  ( rc=0; RUST_TEST_THREADS=$sc_threads "$(cat "$tmp/$n.bin")" -q --exact $(cat "$tmp/$n.names") > "$tmp/$n.out" 2>&1 || rc=$?; echo "$rc" > "$tmp/$n.rc" ) &
  n=$((n + 1))
done
wait
passed=${doc:-0}; failed=0; bad=""
n=0
while [ "$n" -lt "$i" ]; do
  r=$(sed -n 's/^test result: [A-Za-z]*\. \([0-9]*\) passed; \([0-9]*\) failed.*/\1 \2/p' "$tmp/$n.out" | tail -1)
  if [ -n "$r" ]; then passed=$((passed + ${r%% *})); failed=$((failed + ${r##* })); fi
  if [ "$(cat "$tmp/$n.rc")" != 0 ]; then bad="$bad $n"; [ -n "$r" ] || failed=$((failed + 1)); fi
  n=$((n + 1))
done
for n in $bad; do echo "--- $(basename "$(cat "$tmp/$n.bin")"), one of $i processes:"; cat "$tmp/$n.out"; done
took=$(( $(date +%s) - started ))
if [ -n "$bad" ]; then echo "Tests: $passed passed, $failed failed, in $i processes in ${took}s."; exit 101; fi
echo "Tests: $passed passed, in $i processes in ${took}s."

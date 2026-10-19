#!/bin/sh
# A new worktree starts from a warm build, not from nothing: the workspace setup (scripts/setup.sh) copies the last
# kept build of the main line into its fresh `target/quick` before it builds (`ws_seed`), so that build only redoes
# what differs, and keeps its own build as the next one's start when it is newer on the main line (`ws_keep`).
# Sourced, it only defines the two; HERDR_PROJECTS_NO_WARM=1 turns both off, as does a CARGO_TARGET_DIR of your own. How and why: docs/testing.md.
#
# The kept builds live in ~/.cache/herdr-projects/warm/<repo>/, one folder each (`seed-…`: `target/quick`, the
# commit it was built from, the compiler, the folder it was built in) and `current` naming the newest. A copy is a
# clone where the file system can (APFS, Btrfs, XFS), a plain copy elsewhere; never hard links, as cargo writes into
# some of its files in place. Cargo decides what is out of date by the times files were changed, and every file of a
# fresh worktree is new: the files whose content is the kept build's commit's are given an old time, so cargo takes
# them as built; every other file (changed since, or by you) keeps its own and is built again.
ws_old=200001010000
ws_dir() {
  ws_git=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || return 1
  ws_root="${XDG_CACHE_HOME:-$HOME/.cache}/herdr-projects/warm/$(printf '%s' "$ws_git" | cksum | cut -d' ' -f1)"
}
ws_rustc() { rustc -vV 2>/dev/null | tr -d '\r'; }
# A copy of a folder that keeps every file's time: a clone where it can be, else a plain copy.
ws_copy() {
  cp -pR --reflink=auto "$1" "$2" 2>/dev/null && return 0; rm -rf "$2"
  cp -cpR "$1" "$2" 2>/dev/null && return 0; rm -rf "$2"
  cp -pR "$1" "$2"
}
# Tracked files whose content here is not the commit's $1: changed since it, or changed in this folder.
ws_changed() {
  git -c core.quotePath=false diff --name-only --no-renames --diff-filter=d "$1" HEAD -- && git -c core.quotePath=false diff --name-only --no-renames --diff-filter=d HEAD --
}
ws_seed() {
  ws_clean=""
  [ -z "$HERDR_PROJECTS_NO_WARM$CARGO_TARGET_DIR" ] && [ ! -e target/quick ] && ws_dir || return 0
  [ -z "$(git status --porcelain --untracked-files=no 2>/dev/null)" ] && ws_clean=$(git rev-parse HEAD) || ws_clean=""
  ws_name=$(cat "$ws_root/current" 2>/dev/null) && [ -n "$ws_name" ] && [ -d "$ws_root/$ws_name/target/quick" ] || return 0
  ws_src="$ws_root/$ws_name"
  [ "$(cat "$ws_src/rustc" 2>/dev/null)" = "$(ws_rustc)" ] || return 0
  # Cargo never removes what it no longer uses (the libraries of older versions): a build grown past 2 GB is not copied,
  # and the one built from nothing here is kept in its place.
  [ "$(du -sk "$ws_src/target/quick" 2>/dev/null | cut -f1)" -lt 2097152 ] 2>/dev/null || return 0
  ws_from=$(cat "$ws_src/commit") && ws_was=$(cat "$ws_src/folder") || return 0
  ws_list=$(ws_changed "$ws_from" 2>/dev/null) || ws_list="-"
  mkdir -p target && rm -rf target/quick.warm.$$ || return 0
  if ! ws_copy "$ws_src/target/quick" target/quick.warm.$$ || ! mv target/quick.warm.$$ target/quick; then
    rm -rf target/quick.warm.$$ target/quick; echo "Could not copy the warm build: this worktree builds from nothing." >&2; return 0
  fi
  # What was built with that folder's path written into it (env!("CARGO_MANIFEST_DIR")) cargo would take as built
  # here too: built again (its notes removed), never carried over. A path into its own target/ cargo moves itself.
  for d in target/quick/deps/*.d target/quick/build/*/*.d; do
    grep '^# env-dep:' "$d" 2>/dev/null | grep -F "$ws_was" | grep -vF "$ws_was/target/" | grep -q . || continue
    h=${d##*-}; rm -rf target/quick/.fingerprint/*-"${h%.d}"
  done
  if [ "$ws_list" != "-" ]; then
    git ls-files -z | xargs -0 touch -h -t "$ws_old" 2>/dev/null || true
    printf '%s\n' "$ws_list" | while IFS= read -r f; do [ -z "$f" ] || touch -h -- "$f" 2>/dev/null || true; done
  fi
  echo "Started from the warm build of ${ws_from%"${ws_from#???????}"}: only what changed since is built again."
}
ws_keep() {
  [ -z "$HERDR_PROJECTS_NO_WARM$CARGO_TARGET_DIR" ] && [ -n "$ws_clean" ] && [ -d target/quick ] && ws_dir || return 0
  # Kept only as built: from a commit of the main line, with nothing changed before or during the build.
  [ "$(git rev-parse HEAD)" = "$ws_clean" ] && [ -z "$(git status --porcelain --untracked-files=no)" ] || return 0
  ws_main=""
  for b in refs/remotes/origin/HEAD refs/heads/main refs/heads/master; do
    git merge-base --is-ancestor HEAD "$b" 2>/dev/null && { ws_main=1; break; }
  done
  [ -n "$ws_main" ] || return 0
  ws_name=$(cat "$ws_root/current" 2>/dev/null) || ws_name=""
  if [ -n "$ws_name" ] && [ -d "$ws_root/$ws_name" ] && [ "$(cat "$ws_root/$ws_name/rustc" 2>/dev/null)" = "$(ws_rustc)" ]; then
    ws_had=$(cat "$ws_root/$ws_name/commit" 2>/dev/null)
    # Only forward: a build of the same commit or of an older one is no warmer (one off the line is replaced).
    [ "$ws_had" != "$ws_clean" ] && ! git merge-base --is-ancestor HEAD "$ws_had" 2>/dev/null || return 0
  fi
  ws_new="$ws_root/new.$$"
  rm -rf "$ws_new" && mkdir -p "$ws_new/target" || return 0
  if ws_copy target/quick "$ws_new/target/quick" && echo "$ws_clean" > "$ws_new/commit" && pwd -P > "$ws_new/folder" \
    && ws_rustc > "$ws_new/rustc" && mv "$ws_new" "$ws_root/seed-$ws_clean-$$" \
    && echo "seed-$ws_clean-$$" > "$ws_root/current.$$" && mv "$ws_root/current.$$" "$ws_root/current"; then
    [ -z "$ws_name" ] || touch "$ws_root/$ws_name/retired" 2>/dev/null || true
  else rm -rf "$ws_new" "$ws_root/current.$$"; fi
  ws_tidy
}
# The kept builds no longer current go 30 minutes after they stopped being (a worktree may be copying one); a copy
# left half made, after an hour.
ws_tidy() {
  ws_name=$(cat "$ws_root/current" 2>/dev/null) || return 0
  for s in "$ws_root"/seed-*; do
    [ -d "$s" ] && [ "${s##*/}" != "$ws_name" ] || continue
    [ -e "$s/retired" ] || { touch "$s/retired" 2>/dev/null || true; continue; }
    [ -n "$(find "$s/retired" -mmin +30 2>/dev/null)" ] && rm -rf "$s"
  done
  find "$ws_root" -maxdepth 1 -name 'new.*' -mmin +60 -exec rm -rf {} + 2>/dev/null || true
}

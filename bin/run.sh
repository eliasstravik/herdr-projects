#!/bin/sh
# herdr runs every plugin command through this file, with its server's environment (which may lack the user's PATH).
# It runs the program, the first of:
#   1. the one built from this checkout (target/release/herdr-projects), when no source file is newer than it;
#   2. the downloaded one for this platform, bin/herdr-projects-<os>-<arch>, when it is this checkout's version
#      (Cargo.toml) or newer (`herdr-projects update` puts a newer one there); its version is in the .version file
#      beside it, read without running it;
#   3. that file, downloaded now from the GitHub release of this checkout's version: its checksum checked, renamed into
#      place in one step, one download at a time (a second start waits for the first);
#   4. failing that: an older build from this checkout, or an older download, with a line saying so. A failed
#      download is tried again ten minutes later, not on every command.
# When there is none it says on stderr what to do and exits 1: it never waits long, so it never holds herdr up.
# `run.sh --fetch` only gets the program (the plugin's build step), building it from source when it cannot download.
# HERDR_PROJECTS_RELEASES names another place for the releases (a fork, a mirror, a folder as file:///…).
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(dirname -- "$here")
if [ -f "$root/herdr-control/mission-control" ]; then exec sh "$here/source.sh" "$@"; fi
PATH="${PATH:+$PATH:}/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin:$HOME/.cargo/bin"; export PATH
say() { printf 'herdr-projects: %s\n' "$*" >&2; }
fetch_only=""; [ "$1" = "--fetch" ] && { fetch_only=1; shift; }
run() { [ -n "$fetch_only" ] && exit 0; exec "$@"; }

built="$root/target/release/herdr-projects"
fresh() { [ -x "$built" ] && [ -z "$(find "$root/src" "$root/crates" "$root/Cargo.toml" "$root/Cargo.lock" -newer "$built" 2>/dev/null | head -n 1)" ]; }
fresh && run "$built" "$@"

want=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' "$root/Cargo.toml" 2>/dev/null | head -n 1)
case "$(uname -s)" in Linux) os=linux ;; Darwin) os=macos ;; *) os="" ;; esac
case "$(uname -m)" in x86_64 | amd64) arch=x86_64 ;; aarch64 | arm64) arch=aarch64 ;; *) arch="" ;; esac
name="herdr-projects-$os-$arch"
prog="$here/$name"
base="${HERDR_PROJECTS_RELEASES:-https://github.com/eliasstravik/herdr-projects/releases}"; base="${base%/}"

# at_least A B: A is B or newer, part by part as numbers (0.10.0 is newer than 0.9.1).
at_least() {
  a=$1; b=$2
  while [ -n "$a$b" ]; do
    x=${a%%.*}; y=${b%%.*}; x=${x%%[!0-9]*}; y=${y%%[!0-9]*}
    [ "${x:-0}" -gt "${y:-0}" ] && return 0
    [ "${x:-0}" -lt "${y:-0}" ] && return 1
    case $a in *.*) a=${a#*.} ;; *) a="" ;; esac
    case $b in *.*) b=${b#*.} ;; *) b="" ;; esac
  done
  return 0
}
have() { [ -x "$prog" ] && v=$(cat "$prog.version" 2>/dev/null) && [ -n "$v" ] && at_least "$v" "$want"; }
[ -n "$os" ] && [ -n "$arch" ] && have && run "$prog" "$@"

gh_repo="https://github.com/eliasstravik/herdr-projects/releases/download/"
fetch() {
  if command -v curl >/dev/null 2>&1; then curl -fsSL --connect-timeout 10 --max-time 300 -o "$2" "$1" 2>/dev/null && return 0
  elif command -v wget >/dev/null 2>&1; then wget -q --tries=1 -T 30 -O "$2" "$1" && return 0
  else say "neither curl nor wget is installed: install one, or build the program (cd '$root' && cargo build --release)"; return 1; fi
  # A private repo's release is not handed out without a login: gh, logged in, fetches it.
  case "$1" in "$gh_repo"*/*) ;; *) return 1 ;; esac
  command -v gh >/dev/null 2>&1 || return 1
  rest=${1#"$gh_repo"}
  gh release download "${rest%%/*}" -R eliasstravik/herdr-projects -p "${rest#*/}" -O "$2" --clobber </dev/null >/dev/null 2>&1
}
sha256() { { sha256sum "$1" || shasum -a 256 "$1" || openssl dgst -sha256 -r "$1"; } 2>/dev/null | cut -d ' ' -f 1; }
build_hint="or build it here: cd '$root' && cargo build --release (Rust: https://rustup.rs)"

download() {
  if [ -z "$os" ] || [ -z "$arch" ]; then say "no release is made for $(uname -s) $(uname -m): build it here: cd '$root' && cargo build --release (Rust: https://rustup.rs)"; return 1; fi
  [ -n "$want" ] || { say "cannot read the version in $root/Cargo.toml: $build_hint"; return 1; }
  [ -w "$here" ] || { say "cannot write in $here to download the program: $build_hint"; return 1; }
  lock="$prog.lock"; n=0
  until mkdir "$lock" 2>/dev/null; do
    pid=$(cat "$lock/pid" 2>/dev/null)
    if { [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; } || { [ -z "$pid" ] && [ "$n" -ge 5 ]; }; then rm -rf "$lock"; continue; fi
    n=$((n + 1))
    [ "$n" -ge 150 ] && { say "another start is still downloading $name: try again in a minute"; return 1; }
    sleep 1
    have && return 0 # the other start got it
  done
  echo $$ > "$lock/pid"
  tmp="$here/.$name.$$.download"
  trap 'rm -rf "$lock" "$tmp" "$tmp.sha256"' EXIT
  trap 'exit 1' INT TERM HUP
  url="$base/download/v$want/$name"
  say "downloading herdr-projects $want for $os $arch (once)"
  ok=""
  if ! fetch "$url" "$tmp" || ! fetch "$url.sha256" "$tmp.sha256"; then
    say "could not download $url (no network, no release v$want with this file yet, or a private repo: log in with gh auth login): try again later, $build_hint"
  elif sum=$(cut -d ' ' -f 1 < "$tmp.sha256") && got=$(sha256 "$tmp") && [ -z "$got" ]; then
    say "no sha256sum, shasum or openssl here to check the download: install one, $build_hint"
  elif [ "$sum" != "$got" ]; then
    say "the download does not match its checksum, so it is not used: try again, $build_hint"
  else
    # Renamed over the old one, never written into: a running program keeps its file, and macOS kills one changed in place.
    chmod 755 "$tmp" && mv -f "$tmp" "$prog" && printf '%s\n' "$want" > "$prog.version.$$" && mv -f "$prog.version.$$" "$prog.version" && ok=1
  fi
  rm -rf "$lock" "$tmp" "$tmp.sha256"; trap - EXIT INT TERM HUP
  [ -n "$ok" ]
}
# A download that failed is not tried again for ten minutes (but by the build step): every hook would wait for it.
failed="$prog.failed"
if [ -z "$fetch_only" ] && [ -n "$(find "$failed" -mmin -10 2>/dev/null)" ]; then false
elif download; then rm -f "$failed"; run "$prog" "$@"
else : > "$failed" 2>/dev/null; fi

if [ -n "$fetch_only" ] && command -v cargo >/dev/null 2>&1; then
  say "building it from source instead: cargo build --release (a few minutes)"
  (cd "$root" && cargo build --release --locked) && exit 0
fi
[ -x "$built" ] && { say "using $built, which is older than its source: build it again (cd '$root' && cargo build --release)"; run "$built" "$@"; }
[ -x "$prog" ] && { say "using the downloaded herdr-projects $(cat "$prog.version" 2>/dev/null), not $want"; run "$prog" "$@"; }
exit 1

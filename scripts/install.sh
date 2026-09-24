#!/bin/sh
# Puts the herdr-projects binary at target/release/herdr-projects.
#
# Herdr runs this as the plugin's build step, and `herdr-projects update` runs
# it in a linked checkout. It downloads the prebuilt binary of the release
# named by herdr-plugin.toml's `version` for this machine, checks it against
# the release's SHA256SUMS, and falls back to `cargo build --release --locked`
# when there is no such binary or it cannot be verified.
#
#   HERDR_PROJECTS_BUILD=source         always build from source
#   HERDR_PROJECTS_DOWNLOAD_URL=<url>   download from <url>/v<version>/ instead
#                                       of the GitHub release of `origin`
set -u

cd "$(dirname "$0")/.." || exit 1

say() { printf 'herdr-projects install: %s\n' "$*" >&2; }

build_from_source() {
  [ -n "${tmp:-}" ] && rm -rf "$tmp"
  say "$1"
  say "building from source instead: cargo build --release --locked (this takes a minute or two)"
  if ! command -v cargo >/dev/null 2>&1; then
    say "cargo is not installed. Install Rust 1.89 or newer (https://rustup.rs) and a C compiler, then install again."
    exit 1
  fi
  exec cargo build --release --locked
}

if [ "${HERDR_PROJECTS_BUILD:-}" = source ]; then
  build_from_source "HERDR_PROJECTS_BUILD=source is set"
fi

version=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' herdr-plugin.toml | head -n 1)
[ -n "$version" ] || build_from_source "herdr-plugin.toml has no version"
tag="v$version"

case "$(uname -s)" in
  Darwin) os=apple-darwin ;;
  Linux) os=unknown-linux-musl ;;
  *) build_from_source "there is no prebuilt binary for $(uname -s)" ;;
esac
case "$(uname -m)" in
  arm64 | aarch64) arch=aarch64 ;;
  x86_64 | amd64) arch=x86_64 ;;
  *) build_from_source "there is no prebuilt binary for $(uname -m)" ;;
esac
asset="herdr-projects-$arch-$os"

# A prebuilt binary matches only the release commit itself. A checkout with
# local changes, or one on a commit after the release, builds what it has.
origin=
if [ -d .git ] || [ -f .git ]; then
  if [ -n "$(git status --porcelain --untracked-files=no 2>/dev/null)" ]; then
    build_from_source "this checkout has uncommitted changes"
  fi
  head=$(git rev-parse HEAD 2>/dev/null)
  release=$(git rev-parse -q --verify "refs/tags/$tag^{commit}" 2>/dev/null)
  if [ -z "$release" ]; then
    # `^{}` is the commit an annotated tag points at; a lightweight tag has none.
    release=$(GIT_TERMINAL_PROMPT=0 git ls-remote origin "refs/tags/$tag" "refs/tags/$tag^{}" 2>/dev/null |
      awk '{ sha = $1 } $2 ~ /\^\{\}$/ { peeled = $1 } END { print (peeled != "" ? peeled : sha) }')
  fi
  if [ -z "$release" ]; then
    build_from_source "could not find the $tag tag here or on origin"
  elif [ "$head" != "$release" ]; then
    build_from_source "this checkout is not the $tag release commit"
  fi
  origin=$(git remote get-url origin 2>/dev/null)
fi

if [ -n "${HERDR_PROJECTS_DOWNLOAD_URL:-}" ]; then
  base="${HERDR_PROJECTS_DOWNLOAD_URL%/}/$tag"
else
  repo=$(printf '%s\n' "$origin" | sed -n 's#^.*github\.com[:/]\([^/]*/[^/]*\)$#\1#p' | sed 's#\.git$##')
  base="https://github.com/${repo:-eliasstravik/herdr-projects}/releases/download/$tag"
fi

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL --retry 2 --connect-timeout 10 -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -q -T 10 -O "$2" "$1"; }
else
  build_from_source "neither curl nor wget is installed"
fi
if command -v sha256sum >/dev/null 2>&1; then
  sha256() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum >/dev/null 2>&1; then
  sha256() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
  build_from_source "neither sha256sum nor shasum is installed to check the download"
fi

mkdir -p target/release
tmp=$(mktemp -d "target/release/.download.XXXXXX") || build_from_source "could not create a download folder"
trap 'rm -rf "$tmp"' EXIT

say "downloading $asset $tag"
fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS" ||
  build_from_source "could not download $base/SHA256SUMS"
expected=$(awk -v name="$asset" '$2 == name || $2 == "*" name { print $1 }' "$tmp/SHA256SUMS")
[ -n "$expected" ] || build_from_source "the $tag release has no $asset"
fetch "$base/$asset" "$tmp/$asset" ||
  build_from_source "could not download $base/$asset"
actual=$(sha256 "$tmp/$asset")
[ "$actual" = "$expected" ] ||
  build_from_source "the downloaded $asset does not match its SHA256SUMS entry (got $actual, expected $expected)"

chmod 755 "$tmp/$asset"
reported=$("$tmp/$asset" --version 2>/dev/null)
case "$reported" in
  "herdr-projects $version" | "herdr-projects $version+"*) ;;
  *) build_from_source "the downloaded binary did not run or is not $version (it said: ${reported:-nothing})" ;;
esac

# Rename, never copy over: macOS kills a binary rewritten in place.
mv -f "$tmp/$asset" target/release/herdr-projects || build_from_source "could not move the binary into target/release"
say "installed the prebuilt $asset $tag"
